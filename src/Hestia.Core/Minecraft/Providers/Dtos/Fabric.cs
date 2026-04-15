using System.Text.Json.Serialization;

namespace Hestia.Core.Minecraft.Providers.Dtos;

public record FabricVersion(
    [property: JsonPropertyName("version")] string Version,
    [property: JsonPropertyName("stable")] bool Stable
);
