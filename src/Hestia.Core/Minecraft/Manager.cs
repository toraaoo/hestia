using System.Text.Json;
using Hestia.Core.Minecraft.Models;
using Hestia.Core.Minecraft.Providers;
using Hestia.Core.Utils;

namespace Hestia.Core.Minecraft;

public class Manager(Java.Manager javaManager, AppDataFileSystem fs)
{
    private static readonly JsonSerializerOptions JsonOptions = new() { WriteIndented = true };

    private static readonly IReadOnlyList<IProvider> Providers = typeof(IProvider).Assembly
        .GetTypes()
        .Where(t => t is { IsAbstract: false, IsInterface: false } && t.IsAssignableTo(typeof(IProvider)))
        .Select(t => (IProvider)Activator.CreateInstance(t)!)
        .ToList();

    private readonly Launcher _launcher = new();

    private sealed class RuntimeState(ServerInstance instance)
    {
        public ServerInstance Instance { get; } = instance;
        public CancellationTokenSource Cts { get; } = new();
        public bool StopRequested { get; set; }
    }

    private static readonly TimeSpan StabilityWindow = TimeSpan.FromSeconds(15);

    private readonly Lock _gate = new();
    private readonly Dictionary<Guid, RuntimeState> _runtime = [];
    private readonly Dictionary<Guid, ServerStatus> _status = [];
    private readonly HashSet<Guid> _rconReady = [];

    public async Task<Server> CreateAsync(Server server, IProgressCallback? callback = null)
    {
        ThrowIfPortConflict(server, exclude: null);

        var provider = FindProvider(server.Type);
        var resolved = await provider.ResolveAsync(server);

        var javaVersion = resolved.MinJavaVersion.ToString();
        if (!javaManager.IsInstalled(javaVersion))
            await javaManager.InstallAsync(javaVersion, callback);

        Directory.CreateDirectory(ResolveServerDir(server));

        await DownloadJarAsync(resolved, callback);
        WriteServerProperties(server);
        WriteEula(server);

        var servers = LoadServers();
        servers.Add(server);
        SaveServers(servers);

        return server;
    }

    public Task DeleteAsync(Guid id)
    {
        ThrowIfRunning(id);

        var servers = LoadServers();
        var server = servers.Find(s => s.Id == id);
        servers.RemoveAll(s => s.Id == id);
        SaveServers(servers);

        if (server is not null)
        {
            var dir = ResolveServerDir(server);
            if (Directory.Exists(dir))
                Directory.Delete(dir, recursive: true);
        }

        return Task.CompletedTask;
    }

    public IEnumerable<Server> List() => LoadServers();

    public Task<Server> UpdateAsync(Server server)
    {
        ThrowIfRunning(server.Id);
        ThrowIfPortConflict(server, exclude: server.Id);

        var servers = LoadServers();
        var index = servers.FindIndex(s => s.Id == server.Id);
        if (index < 0)
            throw new HestiaException($"Server '{server.Id}' not found.");

        servers[index] = server;
        SaveServers(servers);

        return Task.FromResult(server);
    }

    public Task<ServerInstance> StartAsync(Guid id)
    {
        if (IsRunning(id))
            throw new HestiaException($"Server '{id}' is already running.");

        var servers = LoadServers();
        var server = servers.Find(s => s.Id == id)
            ?? throw new HestiaException($"Server '{id}' not found.");

        var javaVersion = ResolveInstalledJavaVersion(server);
        var javaExePath = GetJavaExePath(javaVersion);
        var serverDir = ResolveServerDir(server);

        var instance = _launcher.Launch(server, javaExePath, serverDir);

        lock (_gate)
        {
            if (_runtime.Remove(id, out var old))
            {
                old.Cts.Cancel();
                old.Cts.Dispose();
            }

            _runtime[id] = new RuntimeState(instance);
            _status[id] = ServerStatus.Starting;
            _rconReady.Remove(id);
        }

        _ = RunLifecycleAsync(id, instance);

        return Task.FromResult(instance);
    }

    public async Task StopAsync(Guid id)
    {
        ServerInstance? instance;
        lock (_gate)
        {
            if (!_runtime.TryGetValue(id, out var state))
                return;

            state.StopRequested = true;
            state.Cts.Cancel();
            instance = state.Instance;
        }

        await instance!.StopAsync();
    }

    public bool IsRunning(Guid id)
    {
        lock (_gate)
            return _runtime.TryGetValue(id, out var state) && state.Instance.IsRunning;
    }

    public ServerInstance? GetInstance(Guid id)
    {
        lock (_gate)
            return _runtime.TryGetValue(id, out var state) ? state.Instance : null;
    }

    public ServerStatus GetStatus(Guid id)
    {
        lock (_gate)
            return _status.TryGetValue(id, out var status) ? status : ServerStatus.Stopped;
    }

    public async Task<ServerMetrics> GetMetricsAsync(Guid id)
    {
        var instance = GetInstance(id)
            ?? throw new HestiaException($"Server '{id}' is not running.");
        return await instance.GetMetricsAsync();
    }

    private async Task RunLifecycleAsync(Guid id, ServerInstance instance)
    {
        RuntimeState? state;
        lock (_gate)
        {
            if (!_runtime.TryGetValue(id, out state) || !ReferenceEquals(state.Instance, instance))
                return;
        }

        var ct = state.Cts.Token;

        _ = MarkRunningAfterStabilityAsync(id, instance, ct);
        _ = ProbeRconAsync(id, instance, ct);

        int? exitCode;
        try
        {
            exitCode = await instance.WaitForExitAsync();
        }
        catch
        {
            exitCode = null;
        }

        bool stopRequested;
        lock (_gate)
        {
            if (!_runtime.TryGetValue(id, out state) || !ReferenceEquals(state.Instance, instance))
                return;

            stopRequested = state.StopRequested;

            _runtime.Remove(id);
            _rconReady.Remove(id);
            state.Cts.Cancel();
            state.Cts.Dispose();

            _status[id] = stopRequested ? ServerStatus.Stopped : ServerStatus.Crashed;
        }

        await instance.DisposeAsync();
    }

    private async Task MarkRunningAfterStabilityAsync(Guid id, ServerInstance instance, CancellationToken ct)
    {
        try
        {
            await Task.Delay(StabilityWindow, ct);
        }
        catch (OperationCanceledException)
        {
            return;
        }

        TryMarkRunning(id, instance);
    }

    private void TryMarkRunning(Guid id, ServerInstance instance)
    {
        lock (_gate)
        {
            if (!_runtime.TryGetValue(id, out var state) || !ReferenceEquals(state.Instance, instance))
                return;

            if (!instance.IsRunning)
                return;

            if (_status.TryGetValue(id, out var s) && (s == ServerStatus.Crashed || s == ServerStatus.Stopped))
                return;

            _status[id] = ServerStatus.Running;
        }
    }

    private async Task ProbeRconAsync(Guid id, ServerInstance instance, CancellationToken ct)
    {
        var delay = TimeSpan.FromMilliseconds(200);

        while (!ct.IsCancellationRequested)
        {
            lock (_gate)
            {
                if (!_runtime.TryGetValue(id, out var state) || !ReferenceEquals(state.Instance, instance))
                    return;

                if (_status.TryGetValue(id, out var s) && (s == ServerStatus.Crashed || s == ServerStatus.Stopped))
                    return;

                if (_rconReady.Contains(id))
                    return;
            }

            try
            {
                var task = instance.SendCommandAsync("list");
                var timeoutTask = Task.Delay(TimeSpan.FromSeconds(2), ct);
                var completed = await Task.WhenAny(task, timeoutTask);
                if (completed == timeoutTask)
                {
                    if (ct.IsCancellationRequested)
                        return;
                    throw new TimeoutException("RCON probe timed out.");
                }

                _ = await task;

                lock (_gate)
                {
                    if (_runtime.TryGetValue(id, out var state) && ReferenceEquals(state.Instance, instance))
                        _rconReady.Add(id);
                }

                return;
            }
            catch
            {
                try { await Task.Delay(delay, ct); }
                catch (OperationCanceledException) { return; }

                delay = TimeSpan.FromMilliseconds(Math.Min(delay.TotalMilliseconds * 2, 2000));
            }
        }
    }

    private static IProvider FindProvider(ServerType type) =>
        Providers.FirstOrDefault(p => p.Type == type)
            ?? throw new HestiaException($"No provider registered for server type '{type}'.");

    private async Task DownloadJarAsync(ResolvedServer resolved, IProgressCallback? callback)
    {
        var downloader = new Downloader();
        var jarPath = Path.Combine(ResolveServerDir(resolved.Server), "server.jar");
        await downloader.Download(
            resolved.DownloadUrl,
            jarPath,
            checksum: resolved.Checksum,
            checksumAlgorithm: "sha1",
            callback: callback
        );
    }

    private void WriteServerProperties(Server server)
    {
        var path = Path.Combine(ResolveServerDir(server), "server.properties");
        var updates = new Dictionary<string, string>
        {
            ["server-port"]        = server.Network.Port.ToString(),
            ["max-players"]        = server.Network.MaxPlayers.ToString(),
            ["motd"]               = server.Network.MotD,
            ["view-distance"]      = server.Network.ViewDistance.ToString(),
            ["online-mode"]        = server.Network.OnlineMode.ToString().ToLowerInvariant(),
            ["white-list"]         = server.Network.Whitelist.ToString().ToLowerInvariant(),
            ["enable-rcon"]        = server.Rcon.Enabled.ToString().ToLowerInvariant(),
            ["rcon.port"]          = server.Rcon.Port.ToString(),
            ["rcon.password"]      = server.Rcon.Password,
            ["level-name"]         = server.World.Name,
            ["level-seed"]         = server.World.Seed ?? "",
            ["default-game-mode"]  = WorldPropertyValues.Of(server.World.GameMode),
            ["difficulty"]         = WorldPropertyValues.Of(server.World.Difficulty),
        };
        MergeServerProperties(path, updates);
    }

    private static void MergeServerProperties(string path, Dictionary<string, string> updates)
    {
        var lines = File.Exists(path) ? [..File.ReadAllLines(path)] : new List<string>();
        var written = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        for (var i = 0; i < lines.Count; i++)
        {
            var line = lines[i];
            if (line.StartsWith('#') || !line.Contains('='))
                continue;

            var eq = line.IndexOf('=');
            var key = line[..eq].Trim();

            if (updates.TryGetValue(key, out var value))
            {
                lines[i] = $"{key}={value}";
                written.Add(key);
            }
        }

        foreach (var (key, value) in updates)
        {
            if (!written.Contains(key))
                lines.Add($"{key}={value}");
        }

        File.WriteAllLines(path, lines);
    }

    private void WriteEula(Server server) =>
        File.WriteAllText(Path.Combine(ResolveServerDir(server), "eula.txt"), "eula=true\n");

    private string ResolveServerDir(Server server) =>
        server.Directory ?? fs.Servers.GetServerDir(server.Id);

    private string ResolveInstalledJavaVersion(Server server)
    {
        var installed = javaManager.ListInstalled().ToList();
        if (installed.Count == 0)
            throw new HestiaException("No Java installations found. Create the server first to auto-install Java.");

        return installed
            .Select(name => name.Replace("jdk-", string.Empty))
            .OrderByDescending(v => int.TryParse(v, out var n) ? n : 0)
            .First();
    }

    private string GetJavaExePath(string version)
    {
        var dir = fs.Java.GetInstallationDir(version);
        var exe = RuntimeInfo.Current.Os == System.Runtime.InteropServices.OSPlatform.Windows
            ? "java.exe"
            : "java";

        var bin = FindJavaExecutable(dir, exe)
            ?? throw new HestiaException($"Could not locate Java executable in '{dir}'.");

        return bin;
    }

    private static string? FindJavaExecutable(string root, string exe)
    {
        if (!Directory.Exists(root))
            return null;

        var direct = Path.Combine(root, "bin", exe);
        if (File.Exists(direct))
            return direct;

        return Directory.GetDirectories(root).Select(subdir => Path.Combine(subdir, "bin", exe)).FirstOrDefault(nested => File.Exists(nested));
    }

    private void ThrowIfRunning(Guid id)
    {
        if (IsRunning(id))
            throw new HestiaException($"Cannot modify server '{id}' while it is running.");
    }

    private void ThrowIfPortConflict(Server server, Guid? exclude)
    {
        var existing = LoadServers().Where(s => s.Id != exclude);

        foreach (var other in existing)
        {
            if (other.Network.Port == server.Network.Port)
                throw new HestiaException(
                    $"Port {server.Network.Port} is already used by server '{other.Name}' ({other.Id}).");

            if (server.Rcon.Enabled && other.Rcon.Enabled && other.Rcon.Port == server.Rcon.Port)
                throw new HestiaException(
                    $"RCON port {server.Rcon.Port} is already used by server '{other.Name}' ({other.Id}).");

            if (server.Rcon.Enabled && other.Network.Port == server.Rcon.Port)
                throw new HestiaException(
                    $"RCON port {server.Rcon.Port} conflicts with the game port of server '{other.Name}' ({other.Id}).");

            if (other.Rcon.Enabled && other.Rcon.Port == server.Network.Port)
                throw new HestiaException(
                    $"Port {server.Network.Port} conflicts with the RCON port of server '{other.Name}' ({other.Id}).");
        }
    }

    private List<Server> LoadServers()
    {
        var path = fs.Servers.ServersJsonPath;
        if (!File.Exists(path))
            return [];

        var json = File.ReadAllText(path);
        return JsonSerializer.Deserialize<List<Server>>(json, JsonOptions) ?? [];
    }

    private void SaveServers(List<Server> servers)
    {
        var json = JsonSerializer.Serialize(servers, JsonOptions);
        File.WriteAllText(fs.Servers.ServersJsonPath, json);
    }
}
