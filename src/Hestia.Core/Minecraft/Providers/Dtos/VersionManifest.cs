using System.Text.Json.Serialization;

namespace Hestia.Core.Minecraft.Providers.Dtos;

public record VersionManifest(
    [property: JsonPropertyName("versions")] List<VersionEntry> Versions
);

public record VersionEntry(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("type")] string Type,
    [property: JsonPropertyName("url")] string Url
);
