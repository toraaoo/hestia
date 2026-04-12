using System.Diagnostics;
using System.Runtime.CompilerServices;
using Hestia.Core.Abstractions;
using Hestia.Core.Rcon;
using Hestia.Core.Server;

namespace Hestia.Core.Monitoring;

public sealed class Monitor : IServerMonitor
{
    private readonly IServerManager _serverManager;
    private readonly IRconService _rconService;
    private readonly IEventBus _eventBus;

    public Monitor(
        IServerManager serverManager,
        IRconService rconService,
        IEventBus eventBus)
    {
        _serverManager = serverManager;
        _rconService = rconService;
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

    private async Task<ServerStatus> SampleStatusAsync(
        Server.MinecraftServer server,
        CancellationToken ct)
    {
        if (server.State != ServerState.Running)
        {
            return new ServerStatus(
                ServerId: server.Id,
                State: server.State,
                PlayerCount: 0,
                MaxPlayers: server.Options.MaxPlayers,
                OnlinePlayers: [],
                Tps: null,
                Resources: null,
                Uptime: null);
        }

        var resources = SampleProcessResources(server);
        var (players, tps, uptime) = await SampleRconDataAsync(server, ct).ConfigureAwait(false);

        return new ServerStatus(
            ServerId: server.Id,
            State: server.State,
            PlayerCount: players.Count,
            MaxPlayers: server.Options.MaxPlayers,
            OnlinePlayers: players,
            Tps: tps,
            Resources: resources,
            Uptime: uptime);
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

    private async Task<(IReadOnlyList<PlayerInfo> Players, double? Tps, TimeSpan? Uptime)>
        SampleRconDataAsync(Server.MinecraftServer server, CancellationToken ct)
    {
        if (!server.RconOptions.Enabled)
            return ([], null, null);

        var credentials = new RconCredentials(
            "127.0.0.1",
            server.RconOptions.Port,
            server.RconOptions.Password);

        RconConnection? conn = null;
        try
        {
            conn = await _rconService.ConnectAsync(server.Id, credentials, ct).ConfigureAwait(false);

            var listResponse = await _rconService.SendCommandAsync(conn.Id, "list", ct)
                .ConfigureAwait(false);
            var players = ParsePlayerList(listResponse.Payload);

            double? tps = null;
            if (server.Type != ServerType.Vanilla)
            {
                var tpsResponse = await _rconService.SendCommandAsync(conn.Id, "tps", ct)
                    .ConfigureAwait(false);
                tps = ParseTps(tpsResponse.Payload);
            }

            return (players, tps, null);
        }
        catch
        {
            return ([], null, null);
        }
        finally
        {
            if (conn is not null)
                await _rconService.DisconnectAsync(conn.Id, CancellationToken.None)
                    .ConfigureAwait(false);
        }
    }

    private static IReadOnlyList<PlayerInfo> ParsePlayerList(string response)
    {
        var colonIdx = response.LastIndexOf(':');
        if (colonIdx < 0) return [];

        var names = response[(colonIdx + 1)..]
            .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);

        return names
            .Select(n => new PlayerInfo(n, Guid.Empty))
            .ToList()
            .AsReadOnly();
    }

    private static double? ParseTps(string response)
    {
        var match = System.Text.RegularExpressions.Regex.Match(
            response, @"[\*]?(\d+\.?\d*)");
        if (!match.Success) return null;
        return double.TryParse(match.Groups[1].Value,
            System.Globalization.CultureInfo.InvariantCulture, out var v)
            ? v : null;
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
