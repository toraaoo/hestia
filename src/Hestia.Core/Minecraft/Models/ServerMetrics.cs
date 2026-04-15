namespace Hestia.Core.Minecraft.Models;

public record ServerMetrics
{
    public required int CurrentPlayers { get; init; }
    public required int MaxPlayers { get; init; }
    public double? Tps { get; init; }
    public required TimeSpan Uptime { get; init; }
    public required string ConnectUrl { get; init; }
}
