using Hestia.Core.Server;

namespace Hestia.Core.Abstractions;

public interface IServerManager
{
    ValueTask<IReadOnlyList<MinecraftServer>> GetServersAsync(CancellationToken ct = default);

    ValueTask<MinecraftServer?> GetServerAsync(Guid serverId, CancellationToken ct = default);

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

    IAsyncEnumerable<string> StreamConsoleOutputAsync(
        Guid serverId,
        CancellationToken ct = default);

    ValueTask<(DateTimeOffset StartedAt, int ProcessId)?> GetRuntimeInfoAsync(
        Guid serverId,
        CancellationToken ct = default);

    IReadOnlyList<string> GetOnlinePlayers(Guid serverId);
}
