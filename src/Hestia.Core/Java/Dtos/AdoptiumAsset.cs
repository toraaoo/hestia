using System.Text.Json.Serialization;

namespace Hestia.Core.Java.Dtos;

public class AdoptiumAsset
{
    [JsonPropertyName("binary")]
    public AdoptiumBinary Binary { get; set; } = new();

    [JsonPropertyName("release_link")]
    public string ReleaseLink { get; set; } = "";

    [JsonPropertyName("release_name")]
    public string ReleaseName { get; set; } = "";

    [JsonPropertyName("vendor")]
    public string Vendor { get; set; } = "";

    [JsonPropertyName("version")]
    public JavaVersion Version { get; set; } = new();
}
