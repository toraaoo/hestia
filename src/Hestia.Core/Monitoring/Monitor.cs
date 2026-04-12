using System.Diagnostics;
using System.Runtime.CompilerServices;
using Hestia.Core.Abstractions;
using Hestia.Core.Server;

namespace Hestia.Core.Monitoring;

public sealed class Monitor : IServerMonitor
{
    private readonly IServerManager _serverManager;
    private readonly IEventBus _eventBus;

    public Monitor(
        IServerManager serverManager,
        IEventBus eventBus)
    {
        _serverManager = serverManager;
        _eventBus = eventBus;
    }

    public async ValueTask<ServerStatus> GetStatusAsync(
        Guid serverId,
        CancellationToken ct = default)
    {
        var server = await _serverManager.GetServerAsync(serverId, ct).ConfigureAwait(false);
        if (server is null)
            throw new KeyNotFoundException($"Server '{serverId}' not found.");

        return await SampleStatusAsync(server, ct).ConfigureAwait(false);
    }

    public IAsyncDisposable StartMonitoring(
        Guid serverId,
        TimeSpan interval,
        CancellationToken ct = default)
    {
        var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        var task = Task.Run(async () =>
        {
            await foreach (var status in WatchStatusAsync(serverId, interval, cts.Token)
                               .ConfigureAwait(false))
            {
                await _eventBus.PublishAsync(
                    new ServerStatusUpdatedEvent(serverId, status), cts.Token)
                    .ConfigureAwait(false);
            }
        }, CancellationToken.None);

        return new MonitorHandle(cts, task);
    }

    public async IAsyncEnumerable<ServerStatus> WatchStatusAsync(
        Guid serverId,
        TimeSpan interval,
        [EnumeratorCancellation] CancellationToken ct = default)
    {
        while (!ct.IsCancellationRequested)
        {
            ServerStatus status;
            try
            {
                status = await GetStatusAsync(serverId, ct).ConfigureAwait(false);
            }
            catch (OperationCanceledException) { yield break; }
            catch
            {
                await Task.Delay(interval, ct).ConfigureAwait(false);
                continue;
            }

            yield return status;
            await Task.Delay(interval, ct).ConfigureAwait(false);
        }
    }

    private Task<ServerStatus> SampleStatusAsync(
        Server.MinecraftServer server,
        CancellationToken ct)
    {
        if (server.State != ServerState.Running)
        {
            return Task.FromResult(new ServerStatus(
                ServerId: server.Id,
                State: server.State,
                PlayerCount: 0,
                MaxPlayers: server.Options.MaxPlayers,
                OnlinePlayers: [],
                Tps: null,
                Resources: null,
                Uptime: null));
        }

        var resources = SampleProcessResources(server);

        // Intentionally avoid RCON polling here. It causes the Minecraft server to spam
        // "RCON Client ... started/shutting down" logs due to frequent connect/disconnect.
        return Task.FromResult(new ServerStatus(
            ServerId: server.Id,
            State: server.State,
            PlayerCount: 0,
            MaxPlayers: server.Options.MaxPlayers,
            OnlinePlayers: [],
            Tps: null,
            Resources: resources,
            Uptime: null));
    }

    private static ResourceUsage? SampleProcessResources(Server.MinecraftServer server)
    {
        try
        {
            var processes = Process.GetProcessesByName("java");
            foreach (var proc in processes)
            {
                try
                {
                    proc.Refresh();
                    var memBytes = proc.WorkingSet64;
                    return new ResourceUsage(
                        CpuPercent: 0.0,
                        MemoryBytes: memBytes,
                        MemoryLimitBytes: 0);
                }
                catch { }
                finally { proc.Dispose(); }
            }
        }
        catch { }
        return null;
    }

    private sealed class MonitorHandle(CancellationTokenSource cts, Task task) : IAsyncDisposable
    {
        public async ValueTask DisposeAsync()
        {
            await cts.CancelAsync().ConfigureAwait(false);
            try { await task.ConfigureAwait(false); } catch { }
            cts.Dispose();
        }
    }
}
