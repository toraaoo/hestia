using System.Text.Json.Serialization;

namespace Hestia.Core.Java.Dtos;

public class AdoptiumPackage
{
    [JsonPropertyName("link")]
    public string Link { get; set; } = "";

    [JsonPropertyName("size")]
    public long Size { get; set; }
}
