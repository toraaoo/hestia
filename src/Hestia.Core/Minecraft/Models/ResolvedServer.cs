namespace Hestia.Core.Minecraft.Models;

public record ResolvedServer
{
    public required Server Server { get; init; }
    public required string DownloadUrl { get; init; }
    public required string Checksum { get; init; }
    public required int MinJavaVersion { get; init; }
}
