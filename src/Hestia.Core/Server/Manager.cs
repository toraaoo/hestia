using System.Collections.Concurrent;
using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading.Channels;
using Hestia.Core.Abstractions;
using Hestia.Core.Jre;

namespace Hestia.Core.Server;

public sealed class Manager : IServerManager, IAsyncDisposable
{
    private readonly string _appDataDir;
    private readonly IJreManager _jreManager;
    private readonly IReadOnlyDictionary<ServerType, IServerProvider> _providers;
    private readonly IEventBus _eventBus;
    private readonly string _serversFile;
    private readonly SemaphoreSlim _persistLock = new(1, 1);

    private readonly ConcurrentDictionary<Guid, MinecraftServer> _servers = new();
    private readonly ConcurrentDictionary<Guid, RunningServerContext> _running = new();

    private static readonly TimeSpan DefaultGracePeriod = TimeSpan.FromSeconds(30);

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() }
    };

    public Manager(
        string appDataDir,
        IJreManager jreManager,
        IEnumerable<IServerProvider> providers,
        IEventBus eventBus)
    {
        _appDataDir = appDataDir;
        _jreManager = jreManager;
        _providers = providers.ToDictionary(p => p.ServerType);
        _eventBus = eventBus;
        _serversFile = Path.Combine(appDataDir, "servers.json");
        LoadPersistedServers();
    }

    public ValueTask<IReadOnlyList<MinecraftServer>> GetServersAsync(CancellationToken ct = default)
    {
        IReadOnlyList<MinecraftServer> list = _servers.Values.ToList().AsReadOnly();
        return ValueTask.FromResult(list);
    }

    public ValueTask<MinecraftServer?> GetServerAsync(Guid serverId, CancellationToken ct = default)
    {
        _servers.TryGetValue(serverId, out var server);
        return ValueTask.FromResult(server);
    }

    public async ValueTask<MinecraftServer> CreateServerAsync(
        CreateServerOptions opts,
        IProgress<double>? progress = null,
        CancellationToken ct = default)
    {
        if (!_providers.TryGetValue(opts.Type, out var provider))
            throw new NotSupportedException(
                $"No server provider registered for type '{opts.Type}'.");

        var options = opts.Options ?? new ServerOptions(opts.ServerDirectory);
        var rconOptions = opts.RconOptions ?? new RconOptions();
        var jvmOptions = opts.JvmOptions ?? new JvmOptions();

        var javaRuntimeId = opts.JavaRuntimeId
            ?? (await _jreManager.ResolveRuntimeForVersionAsync(opts.MinecraftVersion, ct)
                .ConfigureAwait(false))?.Id
            ?? throw new InvalidOperationException(
                $"No suitable Java runtime found for Minecraft {opts.MinecraftVersion}. " +
                "Install a JRE first.");

        Directory.CreateDirectory(options.ServerDirectory);

        var jarPath = Path.Combine(options.ServerDirectory, "server.jar");
        await provider.DownloadServerJarAsync(opts.MinecraftVersion, jarPath, progress, ct)
            .ConfigureAwait(false);

        var eulaState = opts.AcceptEula ? EulaState.Accepted : EulaState.Pending;
        if (opts.AcceptEula)
            await File.WriteAllTextAsync(
                Path.Combine(options.ServerDirectory, "eula.txt"),
                "eula=true\n", ct).ConfigureAwait(false);

        WriteServerProperties(options, rconOptions);

        var server = new MinecraftServer(
            Id: Guid.NewGuid(),
            Name: opts.Name,
            MinecraftVersion: opts.MinecraftVersion,
            Type: opts.Type,
            State: ServerState.Stopped,
            EulaState: eulaState,
            Options: options,
            RconOptions: rconOptions,
            JvmOptions: jvmOptions,
            JavaRuntimeId: javaRuntimeId);

        _servers[server.Id] = server;
        await PersistAsync(ct).ConfigureAwait(false);
        await _eventBus.PublishAsync(new ServerCreatedEvent(server), ct).ConfigureAwait(false);

        return server;
    }

    public async ValueTask DeleteServerAsync(Guid serverId, CancellationToken ct = default)
    {
        if (_running.ContainsKey(serverId))
            await StopServerAsync(serverId, ct: ct).ConfigureAwait(false);

        if (!_servers.TryRemove(serverId, out var server))
            return;

        if (Directory.Exists(server.Options.ServerDirectory))
            Directory.Delete(server.Options.ServerDirectory, recursive: true);

        await PersistAsync(ct).ConfigureAwait(false);
        await _eventBus.PublishAsync(new ServerDeletedEvent(serverId), ct).ConfigureAwait(false);
    }

    public async ValueTask AcceptEulaAsync(Guid serverId, CancellationToken ct = default)
    {
        var server = RequireServer(serverId);
        if (server.EulaState == EulaState.Accepted) return;

        await File.WriteAllTextAsync(
            Path.Combine(server.Options.ServerDirectory, "eula.txt"),
            "eula=true\n", ct).ConfigureAwait(false);

        UpdateServer(serverId, s => s with { EulaState = EulaState.Accepted });
        await PersistAsync(ct).ConfigureAwait(false);
    }

    public async ValueTask StartServerAsync(Guid serverId, CancellationToken ct = default)
    {
        var server = RequireServer(serverId);

        if (server.EulaState != EulaState.Accepted)
            throw new InvalidOperationException(
                $"Server '{server.Name}' cannot start: EULA has not been accepted. " +
                "Call AcceptEulaAsync first.");

        if (_running.ContainsKey(serverId))
            return;

        var runtime = await ResolveRuntimeAsync(server, ct).ConfigureAwait(false);
        var args = BuildJvmArguments(server, runtime);

        var process = new Process
        {
            StartInfo = new ProcessStartInfo
            {
                FileName = runtime.ExecutablePath,
                Arguments = args,
                WorkingDirectory = server.Options.ServerDirectory,
                RedirectStandardInput = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true,
                StandardOutputEncoding = Encoding.UTF8,
                StandardErrorEncoding = Encoding.UTF8
            },
            EnableRaisingEvents = true
        };

        var consoleChannel = Channel.CreateBounded<string>(new BoundedChannelOptions(4096)
        {
            FullMode = BoundedChannelFullMode.DropOldest,
            SingleWriter = true,
            SingleReader = false,
            AllowSynchronousContinuations = false
        });

        var startedTcs = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var lifetime = CancellationTokenSource.CreateLinkedTokenSource(ct);

        UpdateServer(serverId, s => s with { State = ServerState.Starting });
        await _eventBus.PublishAsync(new ServerStartingEvent(serverId), ct).ConfigureAwait(false);

        process.Start();

        var ctx = new RunningServerContext(
            Process: process,
            ConsoleChannel: consoleChannel,
            LifetimeCts: lifetime,
            StartedAt: DateTimeOffset.UtcNow,
            ReadTask: Task.CompletedTask,
            OnlinePlayers: new ConcurrentDictionary<string, byte>(StringComparer.OrdinalIgnoreCase));

        _running[serverId] = ctx;

        var logFile = PrepareLogFile(server.Options.ServerDirectory);

        var readTask = Task.Run(async () =>
        {
            await ReadConsoleLoopAsync(
                process, serverId, consoleChannel.Writer, startedTcs, logFile, ctx.OnlinePlayers, lifetime.Token)
                .ConfigureAwait(false);
        }, CancellationToken.None);

        _running[serverId] = ctx with { ReadTask = readTask };

        try
        {
            await startedTcs.Task.WaitAsync(TimeSpan.FromMinutes(3), ct).ConfigureAwait(false);
            UpdateServer(serverId, s => s with { State = ServerState.Running });
            await _eventBus.PublishAsync(
                new ServerStartedEvent(serverId, process.Id), ct).ConfigureAwait(false);
        }
        catch (TimeoutException)
        {
            UpdateServer(serverId, s => s with { State = ServerState.Running });
            await _eventBus.PublishAsync(
                new ServerStartedEvent(serverId, process.Id), ct).ConfigureAwait(false);
        }

        _ = Task.Run(async () =>
        {
            await process.WaitForExitAsync(CancellationToken.None).ConfigureAwait(false);
            _running.TryRemove(serverId, out _);
            consoleChannel.Writer.TryComplete();
            lifetime.Cancel();
            lifetime.Dispose();

            var exitCode = process.ExitCode;
            var currentState = _servers.TryGetValue(serverId, out var s) ? s.State : ServerState.Stopped;

            if (currentState == ServerState.Stopping)
            {
                UpdateServer(serverId, sv => sv with { State = ServerState.Stopped });
                await _eventBus.PublishAsync(
                    new ServerStoppedEvent(serverId, exitCode), CancellationToken.None)
                    .ConfigureAwait(false);
            }
            else
            {
                UpdateServer(serverId, sv => sv with { State = ServerState.Crashed });
                await _eventBus.PublishAsync(
                    new ServerCrashedEvent(serverId,
                        $"Process exited unexpectedly with code {exitCode}", exitCode),
                    CancellationToken.None)
                    .ConfigureAwait(false);
            }
        }, CancellationToken.None);
    }

    public async ValueTask StopServerAsync(
        Guid serverId,
        TimeSpan gracePeriod = default,
        CancellationToken ct = default)
    {
        if (!_running.TryGetValue(serverId, out var ctx)) return;

        var grace = gracePeriod == default ? DefaultGracePeriod : gracePeriod;

        UpdateServer(serverId, s => s with { State = ServerState.Stopping });
        await _eventBus.PublishAsync(new ServerStoppingEvent(serverId), ct).ConfigureAwait(false);

        try
        {
            await ctx.Process.StandardInput.WriteLineAsync("stop").ConfigureAwait(false);
        }
        catch
        {
        }

        try
        {
            await ctx.Process.WaitForExitAsync(ct)
                .WaitAsync(grace, ct)
                .ConfigureAwait(false);
        }
        catch (TimeoutException)
        {
            try { ctx.Process.Kill(entireProcessTree: true); } catch { }
        }
    }

    public async IAsyncEnumerable<string> StreamConsoleOutputAsync(
        Guid serverId,
        [EnumeratorCancellation] CancellationToken ct = default)
    {
        if (!_running.TryGetValue(serverId, out var ctx))
            yield break;

        await foreach (var line in ctx.ConsoleChannel.Reader.ReadAllAsync(ct).ConfigureAwait(false))
            yield return line;
    }

    public ValueTask<(DateTimeOffset StartedAt, int ProcessId)?> GetRuntimeInfoAsync(
        Guid serverId,
        CancellationToken ct = default)
    {
        if (_running.TryGetValue(serverId, out var ctx))
            return ValueTask.FromResult<(DateTimeOffset, int)?>(
                (ctx.StartedAt, ctx.Process.Id));
        return ValueTask.FromResult<(DateTimeOffset, int)?>(null);
    }

    public IReadOnlyList<string> GetOnlinePlayers(Guid serverId) =>
        _running.TryGetValue(serverId, out var ctx)
            ? [.. ctx.OnlinePlayers.Keys]
            : [];

    public async ValueTask DisposeAsync()
    {
        foreach (var (id, _) in _running.ToArray())
        {
            try { await StopServerAsync(id).ConfigureAwait(false); } catch { }
        }
    }

    private async Task ReadConsoleLoopAsync(
        Process process,
        Guid serverId,
        ChannelWriter<string> writer,
        TaskCompletionSource startedTcs,
        string logFile,
        ConcurrentDictionary<string, byte> onlinePlayers,
        CancellationToken ct)
    {
        try
        {
            while (!process.StandardOutput.EndOfStream && !ct.IsCancellationRequested)
            {
                var line = await process.StandardOutput.ReadLineAsync(ct).ConfigureAwait(false);
                if (line is null) break;

                writer.TryWrite(line);
                await File.AppendAllTextAsync(logFile, line + Environment.NewLine, ct).ConfigureAwait(false);

                ParsePlayerEvent(line, onlinePlayers);

                if (!startedTcs.Task.IsCompleted &&
                    line.Contains("]: Done", StringComparison.Ordinal))
                {
                    startedTcs.TrySetResult();
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            writer.TryWrite($"[hestia] Console read error: {ex.Message}");
        }
        finally
        {
            startedTcs.TrySetResult();
            writer.TryComplete();
        }
    }

    private static void ParsePlayerEvent(string line, ConcurrentDictionary<string, byte> players)
    {
        var sep = line.IndexOf("]: ", StringComparison.Ordinal);
        if (sep < 0) return;

        var msg = line.AsSpan(sep + 3);
        if (msg.EndsWith(" joined the game", StringComparison.Ordinal))
        {
            var name = msg[..^" joined the game".Length].ToString();
            players.TryAdd(name, 0);
        }
        else if (msg.EndsWith(" left the game", StringComparison.Ordinal))
        {
            var name = msg[..^" left the game".Length].ToString();
            players.TryRemove(name, out _);
        }
    }

    private async Task<JavaRuntime> ResolveRuntimeAsync(MinecraftServer server, CancellationToken ct)
    {
        var runtimes = await _jreManager.GetInstalledRuntimesAsync(ct).ConfigureAwait(false);
        return runtimes.FirstOrDefault(r => r.Id == server.JavaRuntimeId)
            ?? await _jreManager.ResolveRuntimeForVersionAsync(server.MinecraftVersion, ct)
                .ConfigureAwait(false)
            ?? throw new InvalidOperationException(
                $"Java runtime '{server.JavaRuntimeId}' not found. Install a JRE first.");
    }

    private static string BuildJvmArguments(MinecraftServer server, JavaRuntime runtime)
    {
        var jvm = server.JvmOptions;
        var sb = new StringBuilder();

        sb.Append($"-Xms{jvm.MinMemory} -Xmx{jvm.MaxMemory} ");

        if (jvm.AdditionalFlags is { Count: > 0 })
            sb.Append(string.Join(' ', jvm.AdditionalFlags)).Append(' ');

        sb.Append("-jar server.jar nogui");
        return sb.ToString();
    }

    private static void WriteServerProperties(ServerOptions opts, RconOptions rcon)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# Generated by Hestia");
        sb.AppendLine($"server-port={opts.Port}");
        sb.AppendLine($"max-players={opts.MaxPlayers}");
        sb.AppendLine($"motd={opts.MotD}");
        sb.AppendLine($"view-distance={opts.ViewDistance}");
        sb.AppendLine($"online-mode={opts.OnlineMode.ToString().ToLowerInvariant()}");
        sb.AppendLine($"white-list={opts.Whitelist.ToString().ToLowerInvariant()}");
        sb.AppendLine($"level-name={opts.LevelName}");
        sb.AppendLine($"difficulty={opts.Difficulty}");
        sb.AppendLine($"enable-rcon={rcon.Enabled.ToString().ToLowerInvariant()}");
        sb.AppendLine($"rcon.port={rcon.Port}");
        sb.AppendLine($"rcon.password={rcon.Password}");

        File.WriteAllText(
            Path.Combine(opts.ServerDirectory, "server.properties"),
            sb.ToString());
    }

    private MinecraftServer RequireServer(Guid serverId)
    {
        if (!_servers.TryGetValue(serverId, out var server))
            throw new KeyNotFoundException($"Server '{serverId}' not found.");
        return server;
    }

    private void UpdateServer(Guid serverId, Func<MinecraftServer, MinecraftServer> update)
    {
        _servers.AddOrUpdate(serverId, id => throw new KeyNotFoundException(),
            (_, existing) => update(existing));
    }

    private void LoadPersistedServers()
    {
        if (!File.Exists(_serversFile)) return;
        try
        {
            var json = File.ReadAllText(_serversFile);
            var list = JsonSerializer.Deserialize<List<MinecraftServer>>(json, JsonOptions);
            if (list is null) return;
            foreach (var s in list)
                _servers[s.Id] = s with { State = ServerState.Stopped };
        }
        catch
        {
        }
    }

    private async Task PersistAsync(CancellationToken ct)
    {
        await _persistLock.WaitAsync(ct).ConfigureAwait(false);
        try
        {
            Directory.CreateDirectory(_appDataDir);
            var json = JsonSerializer.Serialize(_servers.Values.ToList(), JsonOptions);
            await File.WriteAllTextAsync(_serversFile, json, ct).ConfigureAwait(false);
        }
        finally
        {
            _persistLock.Release();
        }
    }

    private static string PrepareLogFile(string serverDirectory)
    {
        var logsDir = Path.Combine(serverDirectory, "logs");
        Directory.CreateDirectory(logsDir);
        return Path.Combine(logsDir, $"{DateTime.UtcNow:yyyyMMdd-HHmmss}.log");
    }

    private sealed record RunningServerContext(
        Process Process,
        Channel<string> ConsoleChannel,
        CancellationTokenSource LifetimeCts,
        DateTimeOffset StartedAt,
        Task ReadTask,
        ConcurrentDictionary<string, byte> OnlinePlayers);
}
