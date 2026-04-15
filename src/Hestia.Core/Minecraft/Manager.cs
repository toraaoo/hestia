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
    private readonly Dictionary<Guid, ServerInstance> _instances = [];

    public async Task<Server> CreateAsync(Server server, IProgressCallback? callback = null)
    {
        ThrowIfPortConflict(server, exclude: null);

        var provider = FindProvider(server.Type);
        var resolved = await provider.ResolveAsync(server);

        var javaVersion = resolved.MinJavaVersion.ToString();
        if (!javaManager.IsInstalled(javaVersion))
            await javaManager.InstallAsync(javaVersion, callback);

        fs.Servers.EnsureServerDir(server.Id);

        await DownloadJarAsync(resolved, callback);
        WriteServerProperties(server);
        WriteEula(server.Id);

        var servers = LoadServers();
        servers.Add(server);
        SaveServers(servers);

        return server;
    }

    public Task DeleteAsync(Guid id)
    {
        ThrowIfRunning(id);

        var servers = LoadServers();
        servers.RemoveAll(s => s.Id == id);
        SaveServers(servers);
        fs.Servers.DeleteServer(id);

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
        var serverDir = fs.Servers.GetServerDir(id);

        var instance = _launcher.Launch(server, javaExePath, serverDir);
        _instances[id] = instance;

        instance.Output.Subscribe(
            onNext: _ => { },
            onCompleted: () => _instances.Remove(id)
        );

        return Task.FromResult(instance);
    }

    public async Task StopAsync(Guid id)
    {
        if (!_instances.TryGetValue(id, out var instance))
            return;

        await instance.StopAsync();
        _instances.Remove(id);
        await instance.DisposeAsync();
    }

    public bool IsRunning(Guid id) => _instances.TryGetValue(id, out var instance) && instance.IsRunning;

    public ServerInstance? GetInstance(Guid id) => _instances.GetValueOrDefault(id);

    public async Task<ServerMetrics> GetMetricsAsync(Guid id)
    {
        var instance = GetInstance(id)
            ?? throw new HestiaException($"Server '{id}' is not running.");
        return await instance.GetMetricsAsync();
    }

    private static IProvider FindProvider(ServerType type) =>
        Providers.FirstOrDefault(p => p.Type == type)
            ?? throw new HestiaException($"No provider registered for server type '{type}'.");

    private async Task DownloadJarAsync(ResolvedServer resolved, IProgressCallback? callback)
    {
        var downloader = new Downloader();
        var jarPath = fs.Servers.GetJarPath(resolved.Server.Id);
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
        var path = fs.Servers.GetPropertiesPath(server.Id);
        var content = $"""
            server-port={server.Port}
            enable-rcon=true
            rcon.port={server.RconPort}
            rcon.password={server.RconPassword}
            level-name={server.World.Name}
            level-seed={server.World.Seed ?? ""}
            default-game-mode={WorldPropertyValues.Of(server.World.GameMode)}
            difficulty={WorldPropertyValues.Of(server.World.Difficulty)}
            """;
        File.WriteAllText(path, content);
    }

    private void WriteEula(Guid id)
    {
        var path = Path.Combine(fs.Servers.GetServerDir(id), "eula.txt");
        File.WriteAllText(path, "eula=true\n");
    }

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
            if (other.Port == server.Port)
                throw new HestiaException(
                    $"Port {server.Port} is already used by server '{other.Name}' ({other.Id}).");

            if (other.RconPort == server.RconPort)
                throw new HestiaException(
                    $"RCON port {server.RconPort} is already used by server '{other.Name}' ({other.Id}).");

            if (other.Port == server.RconPort)
                throw new HestiaException(
                    $"RCON port {server.RconPort} conflicts with the game port of server '{other.Name}' ({other.Id}).");

            if (other.RconPort == server.Port)
                throw new HestiaException(
                    $"Port {server.Port} conflicts with the RCON port of server '{other.Name}' ({other.Id}).");
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
