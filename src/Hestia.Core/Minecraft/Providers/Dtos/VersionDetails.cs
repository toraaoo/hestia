using System.Text.Json.Serialization;

namespace Hestia.Core.Minecraft.Providers.Dtos;

public record VersionDetails(
    [property: JsonPropertyName("downloads")] VersionDownloads Downloads,
    [property: JsonPropertyName("javaVersion")] JavaVersionInfo? JavaVersion
);

public record VersionDownloads(
    [property: JsonPropertyName("server")] DownloadArtifact? Server
);

public record DownloadArtifact(
    [property: JsonPropertyName("url")] string Url,
    [property: JsonPropertyName("sha1")] string Sha1
);

public record JavaVersionInfo(
    [property: JsonPropertyName("majorVersion")] int MajorVersion
);
