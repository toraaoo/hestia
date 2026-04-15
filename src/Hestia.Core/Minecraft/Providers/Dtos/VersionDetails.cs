using System.Text.Json.Serialization;

namespace Hestia.Core.Minecraft.Providers.Dtos;

public record VersionDetails
{
    [JsonPropertyName("downloads")]
    public VersionDownloads Downloads { get; init; } = new();

    [JsonPropertyName("javaVersion")]
    public JavaVersionInfo? JavaVersion { get; init; }
}

public record VersionDownloads
{
    [JsonPropertyName("server")]
    public DownloadArtifact? Server { get; init; }
}

public record DownloadArtifact
{
    [JsonPropertyName("url")]
    public string Url { get; init; } = string.Empty;

    [JsonPropertyName("sha1")]
    public string Sha1 { get; init; } = string.Empty;
}

public record JavaVersionInfo
{
    [JsonPropertyName("majorVersion")]
    public int MajorVersion { get; init; }
}
