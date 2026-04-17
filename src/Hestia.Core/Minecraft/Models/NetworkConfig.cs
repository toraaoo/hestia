namespace Hestia.Core.Minecraft.Models;

public record NetworkConfig
{
    public int Port { get; init; } = 25565;
    public int MaxPlayers { get; init; } = 20;
    public string MotD { get; init; } = "A Minecraft Server";
    public int ViewDistance { get; init; } = 10;
    public bool OnlineMode { get; init; } = true;
    public bool Whitelist { get; init; } = false;
}
