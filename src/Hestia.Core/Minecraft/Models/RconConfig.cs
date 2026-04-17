namespace Hestia.Core.Minecraft.Models;

public record RconConfig
{
    public bool Enabled { get; init; } = true;
    public int Port { get; init; } = 25575;
    public string Password { get; init; } = string.Empty;
    public int TimeoutSeconds { get; init; } = 10;
}
