namespace Hestia.Core.Minecraft.Models;

public enum ServerType
{
    Vanilla,

    // Fabric,
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
    public string Host { get; init; } = "localhost";
    public int Port { get; init; } = 25565;
    public int RconPort { get; init; } = 25575;
    public string RconPassword { get; init; } = string.Empty;
    public List<string> JvmArgs { get; init; } = ["-Xmx2G", "-Xms512M"];
    public WorldConfig World { get; init; } = new();
}
