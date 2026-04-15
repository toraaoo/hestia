using Hestia.Core.Minecraft.Models;

namespace Hestia.Core.Minecraft;

internal static class WorldPropertyValues
{
    internal static string Of(GameMode mode) => mode switch
    {
        GameMode.Creative  => "creative",
        GameMode.Adventure => "adventure",
        GameMode.Spectator => "spectator",
        _                  => "survival",
    };

    internal static string Of(Difficulty difficulty) => difficulty switch
    {
        Difficulty.Peaceful => "peaceful",
        Difficulty.Easy     => "easy",
        Difficulty.Hard     => "hard",
        _                   => "normal",
    };
}
