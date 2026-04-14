using System.Text.Json.Serialization;

namespace Hestia.Core.Java;

public class JavaVersion
{
    [JsonPropertyName("major")]
    public int Major { get; init; }

    [JsonPropertyName("minor")]
    public int Minor { get; init; }

    [JsonPropertyName("security")]
    public int Security { get; init; }

    [JsonPropertyName("patch")]
    public int Patch { get; init; }

    [JsonPropertyName("pre")]
    public string? Pre { get; init; }

    [JsonPropertyName("adopt_build_number")]
    public int AdoptBuildNumber { get; init; }

    [JsonPropertyName("semver")]
    public string Semver { get; init; } = "";

    [JsonPropertyName("openjdk_version")]
    public string OpenJdkVersion { get; init; } = "";

    [JsonPropertyName("build")]
    public int Build { get; init; }

    [JsonPropertyName("optional")]
    public string? Optional { get; init; }
}
