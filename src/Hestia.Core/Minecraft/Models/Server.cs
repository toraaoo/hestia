namespace Hestia.Core.Minecraft.Models;

public enum ServerType
{
    Vanilla,
    Fabric,

    // Paper,
    // Spigot,
    // Forge,
}

public record Server
{
    public Guid Id { get; init; } = Guid.NewGuid();
    public ServerType Type { get; init; } = ServerType.Vanilla;
    public string Name { get; init; } = string.Empty;
    public string Version { get; init; } = string.Empty;
    public string Description { get; init; } = string.Empty;
    public string? Directory { get; init; }
    public string Host { get; init; } = "localhost";
    public NetworkConfig Network { get; init; } = new();
    public RconConfig Rcon { get; init; } = new();
    public JvmConfig Jvm { get; init; } = new();
    public WorldConfig World { get; init; } = new();
}
