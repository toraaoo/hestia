namespace Hestia.Core.Minecraft.Models;

public record JvmConfig
{
    public string MinMemory { get; init; } = "512M";
    public string MaxMemory { get; init; } = "2G";
    public List<string> AdditionalFlags { get; init; } = [];

    public IReadOnlyList<string> BuildArgs() =>
        [$"-Xms{MinMemory}", $"-Xmx{MaxMemory}", ..AdditionalFlags];
}
