using Hestia.Core.Monitoring;

namespace Hestia.Core.Abstractions;

public interface IServerMonitor
{
    ValueTask<ServerStatus> GetStatusAsync(Guid serverId, CancellationToken ct = default);

    IAsyncDisposable StartMonitoring(
        Guid serverId,
        TimeSpan interval,
        CancellationToken ct = default);

    IAsyncEnumerable<ServerStatus> WatchStatusAsync(
        Guid serverId,
        TimeSpan interval,
        CancellationToken ct = default);
}
