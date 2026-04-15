namespace Hestia.Core.Minecraft.Models;

public enum GameMode { Survival, Creative, Adventure, Spectator }

public enum Difficulty { Peaceful, Easy, Normal, Hard }

public record WorldConfig
{
    public string Name { get; init; } = "world";
    public string? Seed { get; init; }
    public GameMode GameMode { get; init; } = GameMode.Survival;
    public Difficulty Difficulty { get; init; } = Difficulty.Normal;
}
