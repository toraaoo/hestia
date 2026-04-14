using System.Text.Json.Serialization;

namespace Hestia.Core.Java.Dtos;

public class ReleaseVersionsResponse
{
    [JsonPropertyName("versions")]
    public List<JavaVersion>? Versions { get; set; }
}
