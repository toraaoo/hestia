using System.Threading.Channels;
using Hestia.Core.Jre;
using Hestia.Core.Monitoring;
using Hestia.Core.Rcon;
using Hestia.Core.Server;

namespace Hestia.Core.Abstractions;

public interface IHestiaService
{
    ValueTask<IReadOnlyList<MinecraftServer>> GetServersAsync(CancellationToken ct = default);

    ValueTask<MinecraftServer?> GetServerAsync(Guid serverId, CancellationToken ct = default);

    ValueTask<IReadOnlyList<string>> GetAvailableVersionsAsync(ServerType type, CancellationToken ct = default);

    ValueTask<MinecraftServer> CreateServerAsync(
        CreateServerOptions options,
        IProgress<double>? progress = null,
        CancellationToken ct = default);

    ValueTask DeleteServerAsync(Guid serverId, CancellationToken ct = default);

    ValueTask AcceptEulaAsync(Guid serverId, CancellationToken ct = default);

    ValueTask StartServerAsync(Guid serverId, CancellationToken ct = default);

    ValueTask StopServerAsync(
        Guid serverId,
        TimeSpan gracePeriod = default,
        CancellationToken ct = default);

    ValueTask RestartServerAsync(
        Guid serverId,
        TimeSpan gracePeriod = default,
        CancellationToken ct = default);

    IAsyncEnumerable<string> StreamLogsAsync(Guid serverId, CancellationToken ct = default);

    ValueTask<RconResponse> SendCommandAsync(
        Guid serverId,
        string command,
        CancellationToken ct = default);

    ValueTask<ServerStatus> GetStatusAsync(Guid serverId, CancellationToken ct = default);

    IAsyncEnumerable<ServerStatus> WatchStatusAsync(
        Guid serverId,
        TimeSpan interval,
        CancellationToken ct = default);

    IAsyncDisposable StartMonitoring(
        Guid serverId,
        TimeSpan interval,
        CancellationToken ct = default);

    ValueTask<IReadOnlyList<JavaRuntime>> GetRuntimesAsync(CancellationToken ct = default);

    ValueTask<JavaRuntime> InstallRuntimeAsync(
        JreInstallOptions options,
        IProgress<double>? progress = null,
        CancellationToken ct = default);

    ChannelReader<TEvent> Subscribe<TEvent>() where TEvent : IHestiaEvent;

    void Unsubscribe<TEvent>(ChannelReader<TEvent> reader) where TEvent : IHestiaEvent;
}
