using System.Net.Http.Json;
using System.Runtime.InteropServices;
using Hestia.Core.Java.Dtos;
using Hestia.Core.Utils;
using Microsoft.AspNetCore.WebUtilities;

namespace Hestia.Core.Java;

public class Resolver
{
    private static readonly HttpClient _http = new();
    private const string AdoptiumBaseUrl = "https://api.adoptopenjdk.net/v3";
    private const string ListVersionsEndpoint = $"{AdoptiumBaseUrl}/info/release_versions";
    private const string ResolveVersionEndpoint = $"{AdoptiumBaseUrl}/assets/latest";

    private static string GetOsApiString(OSPlatform os)
    {
        if (os == OSPlatform.Windows)
            return "windows";
        if (os == OSPlatform.Linux)
            return "linux";
        return os == OSPlatform.OSX ? "mac" : throw new PlatformNotSupportedException($"Unsupported OS: {os}");
    }

    private static string GetArchApiString(Architecture arch) => arch switch
    {
        Architecture.X64 => "x64",
        Architecture.Arm64 => "aarch64",
        _ => throw new PlatformNotSupportedException($"Unsupported architecture: {arch}")
    };

    public async Task<IReadOnlyList<JavaVersion>> ListAvailableAsync(
        int limit = 50,
        int page = 0,
        string vendor = "adoptopenjdk"
    )
    {
        var url = QueryHelpers.AddQueryString(
            ListVersionsEndpoint,
            new Dictionary<string, string?>
            {
                ["lts"] = "true",
                ["page"] = page.ToString(),
                ["page_size"] = limit.ToString(),
                ["project"] = "jdk",
                ["release_type"] = "ga",
                ["semver"] = "false",
                ["sort_method"] = "DEFAULT",
                ["sort_order"] = "DESC",
                ["vendor"] = vendor
            });

        var data = await _http.GetFromJsonAsync<ReleaseVersionsResponse>(url);

        return data?.Versions?
                   .OrderByDescending(v => (v.Major, v.Minor, v.Security, v.Build))
                   .ToList()
               ?? [];
    }

    public async Task<ResolvedJava> ResolveAsync(string version, string vendor = "adoptopenjdk")
    {
        if (string.IsNullOrWhiteSpace(version))
            throw new ArgumentException("Version cannot be empty");

        var url = $"{ResolveVersionEndpoint}/{version}/hotspot?vendor={Uri.EscapeDataString(vendor)}";
        
        Console.WriteLine($"Resolving Java version {version} from {vendor} using URL: {url}");

        var assets = await _http.GetFromJsonAsync<List<AdoptiumAsset>>(url);
        var currentRuntime = RuntimeInfo.Current;
        var osStr = GetOsApiString(currentRuntime.Os);
        var archStr = GetArchApiString(currentRuntime.Arch);

        var asset = assets?
            .FirstOrDefault(a =>
                a.Binary.Os == osStr &&
                a.Binary.Architecture == archStr &&
                a.Binary.ImageType == "jdk");

        if (asset?.Binary is null)
            throw new InvalidOperationException($"No JDK binary found for {osStr} {archStr}");

        return new ResolvedJava
        {
            Version = version,
            DownloadUrl = asset.Binary.Package.Link,
            Checksum = asset.Binary.Checksum,
            SizeBytes = asset.Binary.Package.Size,
            Os = osStr,
            Arch = archStr
        };
    }
}