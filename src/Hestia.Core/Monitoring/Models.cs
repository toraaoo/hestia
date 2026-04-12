using Hestia.Core.Server;

namespace Hestia.Core.Monitoring;

public sealed record ResourceUsage(
    double CpuPercent,
    long MemoryBytes,
    long MemoryLimitBytes);

public sealed record PlayerInfo(string Username, Guid MinecraftUuid);

public sealed record ServerStatus(
    Guid ServerId,
    ServerState State,
    int PlayerCount,
    int MaxPlayers,
    IReadOnlyList<PlayerInfo> OnlinePlayers,
    double? Tps,
    ResourceUsage? Resources,
    TimeSpan? Uptime);
