using System.Text.Json.Serialization;

namespace Hestia.Core.Minecraft.Providers.Dtos;

public record VersionManifest
{
    [JsonPropertyName("versions")]
    public List<VersionEntry> Versions { get; init; } = [];
}

public record VersionEntry
{
    [JsonPropertyName("id")]
    public string Id { get; init; } = string.Empty;

    [JsonPropertyName("type")]
    public string Type { get; init; } = string.Empty;

    [JsonPropertyName("url")]
    public string Url { get; init; } = string.Empty;
}
