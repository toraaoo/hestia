using System.Threading.Channels;
using Hestia.Core.Abstractions;
using Hestia.Core.Jre;
using Hestia.Core.Monitoring;
using Hestia.Core.Rcon;
using Hestia.Core.Server;

namespace Hestia.Core;

public sealed class HestiaService : IHestiaService
{
    private readonly IJreManager _jre;
    private readonly IServerManager _servers;
    private readonly IRconService _rcon;
    private readonly IServerMonitor _monitor;
    private readonly IEventBus _eventBus;
    private readonly IReadOnlyDictionary<ServerType, IServerProvider> _providers;

    public HestiaService(
        IJreManager jre,
        IServerManager servers,
        IRconService rcon,
        IServerMonitor monitor,
        IEventBus eventBus,
        IEnumerable<IServerProvider> providers)
    {
        _jre = jre;
        _servers = servers;
        _rcon = rcon;
        _monitor = monitor;
        _eventBus = eventBus;
        _providers = providers.ToDictionary(p => p.ServerType);
    }

    public ValueTask<IReadOnlyList<MinecraftServer>> GetServersAsync(CancellationToken ct = default)
        => _servers.GetServersAsync(ct);

    public ValueTask<MinecraftServer?> GetServerAsync(Guid serverId, CancellationToken ct = default)
        => _servers.GetServerAsync(serverId, ct);

    public ValueTask<IReadOnlyList<string>> GetAvailableVersionsAsync(ServerType type, CancellationToken ct = default)
    {
        if (!_providers.TryGetValue(type, out var provider))
            throw new NotSupportedException($"No provider registered for server type '{type}'.");
        return provider.GetAvailableVersionsAsync(ct);
    }

    public ValueTask<string> GetLatestVersionAsync(ServerType type, CancellationToken ct = default)
    {
        if (!_providers.TryGetValue(type, out var provider))
            throw new NotSupportedException($"No provider registered for server type '{type}'.");
        return provider.GetLatestVersionAsync(ct);
    }

    public async ValueTask<MinecraftServer> CreateServerAsync(
        CreateServerOptions options,
        IProgress<double>? progress = null,
        CancellationToken ct = default)
    {
        var runtimeId = options.JavaRuntimeId ?? await ResolveOrAcquireRuntimeAsync(options.MinecraftVersion, ct);
        return await _servers.CreateServerAsync(options with { JavaRuntimeId = runtimeId }, progress, ct);
    }

    public ValueTask DeleteServerAsync(Guid serverId, CancellationToken ct = default)
        => _servers.DeleteServerAsync(serverId, ct);

    public ValueTask AcceptEulaAsync(Guid serverId, CancellationToken ct = default)
        => _servers.AcceptEulaAsync(serverId, ct);

    public async ValueTask StartServerAsync(Guid serverId, CancellationToken ct = default)
    {
        var server = await RequireServerAsync(serverId, ct);

        if (server.EulaState != EulaState.Accepted)
            throw new InvalidOperationException(
                $"Server '{server.Name}' cannot start: EULA has not been accepted.");

        var runtimes = await _jre.GetInstalledRuntimesAsync(ct);
        if (!runtimes.Any(r => r.Id == server.JavaRuntimeId))
            throw new InvalidOperationException(
                $"Java runtime '{server.JavaRuntimeId}' no longer exists. Reinstall it first.");

        if (!File.Exists(Path.Combine(server.Options.ServerDirectory, "server.jar")))
            throw new InvalidOperationException(
                $"server.jar not found in '{server.Options.ServerDirectory}'. Recreate the server.");

        await _servers.StartServerAsync(serverId, ct);
    }

    public ValueTask StopServerAsync(Guid serverId, TimeSpan gracePeriod = default, CancellationToken ct = default)
        => _servers.StopServerAsync(serverId, gracePeriod, ct);

    public async ValueTask RestartServerAsync(Guid serverId, TimeSpan gracePeriod = default, CancellationToken ct = default)
    {
        await _servers.StopServerAsync(serverId, gracePeriod, ct);
        await _servers.StartServerAsync(serverId, ct);
    }

    public IAsyncEnumerable<string> StreamLogsAsync(Guid serverId, CancellationToken ct = default)
        => _servers.StreamConsoleOutputAsync(serverId, ct);

    public async ValueTask<RconResponse> SendCommandAsync(Guid serverId, string command, CancellationToken ct = default)
    {
        var server = await RequireServerAsync(serverId, ct);

        if (!server.RconOptions.Enabled)
            throw new InvalidOperationException($"RCON is not enabled for server '{server.Name}'.");

        var credentials = new RconCredentials("127.0.0.1", server.RconOptions.Port, server.RconOptions.Password);

        using var connectCts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        if (server.RconOptions.ConnectTimeoutSeconds > 0)
            connectCts.CancelAfter(TimeSpan.FromSeconds(server.RconOptions.ConnectTimeoutSeconds));

        var conn = await _rcon.ConnectAsync(serverId, credentials, connectCts.Token);
        try
        {
            return await _rcon.SendCommandAsync(conn.Id, command, ct);
        }
        finally
        {
            await _rcon.DisconnectAsync(conn.Id, CancellationToken.None);
        }
    }

    public ValueTask<ServerStatus> GetStatusAsync(Guid serverId, CancellationToken ct = default)
        => _monitor.GetStatusAsync(serverId, ct);

    public IAsyncEnumerable<ServerStatus> WatchStatusAsync(Guid serverId, TimeSpan interval, CancellationToken ct = default)
        => _monitor.WatchStatusAsync(serverId, interval, ct);

    public IAsyncDisposable StartMonitoring(Guid serverId, TimeSpan interval, CancellationToken ct = default)
        => _monitor.StartMonitoring(serverId, interval, ct);

    public ValueTask<IReadOnlyList<JavaRuntime>> GetRuntimesAsync(CancellationToken ct = default)
        => _jre.GetInstalledRuntimesAsync(ct);

    public ValueTask<JavaRuntime> InstallRuntimeAsync(JreInstallOptions options, IProgress<double>? progress = null, CancellationToken ct = default)
        => _jre.InstallRuntimeAsync(options, progress, ct);

    public ChannelReader<TEvent> Subscribe<TEvent>() where TEvent : IHestiaEvent
        => _eventBus.Subscribe<TEvent>();

    public void Unsubscribe<TEvent>(ChannelReader<TEvent> reader) where TEvent : IHestiaEvent
        => _eventBus.Unsubscribe(reader);

    private async ValueTask<MinecraftServer> RequireServerAsync(Guid serverId, CancellationToken ct)
        => await _servers.GetServerAsync(serverId, ct)
           ?? throw new KeyNotFoundException($"Server '{serverId}' not found.");

    private async Task<string> ResolveOrAcquireRuntimeAsync(string minecraftVersion, CancellationToken ct)
    {
        var runtime = await _jre.ResolveRuntimeForVersionAsync(minecraftVersion, ct);
        if (runtime is not null)
            return runtime.Id;

        await foreach (var _ in _jre.DetectSystemRuntimesAsync(ct))
        {
            var candidate = await _jre.ResolveRuntimeForVersionAsync(minecraftVersion, ct);
            if (candidate is not null)
                return candidate.Id;
        }

        var installed = await _jre.InstallRuntimeAsync(
            new JreInstallOptions(RequiredJavaMajor(minecraftVersion)), ct: ct);
        return installed.Id;
    }

    private static int RequiredJavaMajor(string version)
    {
        if (!TryParseVersion(version, out var major, out var minor)) return 21;
        return (major, minor) switch
        {
            (1, >= 20) when minor >= 5 => 21,
            (1, >= 17) => 17,
            _ => 8
        };
    }

    private static bool TryParseVersion(string version, out int major, out int minor)
    {
        major = minor = 0;
        var parts = version.Split('.');
        return parts.Length >= 2
            && int.TryParse(parts[0], out major)
            && int.TryParse(parts[1], out minor);
    }
}
