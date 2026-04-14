using System.Text.Json.Serialization;

namespace Hestia.Core.Java.Dtos;

public class AdoptiumBinary
{
    [JsonPropertyName("os")]
    public string Os { get; set; } = "";

    [JsonPropertyName("architecture")]
    public string Architecture { get; set; } = "";

    [JsonPropertyName("package")]
    public AdoptiumPackage Package { get; set; } = new();

    [JsonPropertyName("checksum")]
    public string Checksum { get; set; } = "";

    [JsonPropertyName("image_type")]
    public string ImageType { get; set; } = "";

    [JsonPropertyName("jvm_impl")]
    public string JvmImpl { get; set; } = "";

    [JsonPropertyName("heap_size")]
    public string HeapSize { get; set; } = "";

    [JsonPropertyName("project")]
    public string Project { get; set; } = "";

    [JsonPropertyName("download_count")]
    public int DownloadCount { get; set; }

    [JsonPropertyName("updated_at")]
    public string UpdatedAt { get; set; } = "";
}
