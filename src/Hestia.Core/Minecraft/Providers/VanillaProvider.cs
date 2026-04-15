using System.Net.Http.Json;
using Hestia.Core.Minecraft.Models;
using Hestia.Core.Minecraft.Providers.Dtos;

namespace Hestia.Core.Minecraft.Providers;

public sealed class VanillaProvider : IProvider
{
    private const string ManifestUrl = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

    private readonly HttpClient _http = new();

    public ServerType Type => ServerType.Vanilla;

    public async Task<List<MinecraftVersion>> GetVersionsAsync()
    {
        var manifest = await _http.GetFromJsonAsync<VersionManifest>(ManifestUrl)
            ?? throw new HestiaException("Failed to fetch Minecraft version manifest.");

        return manifest.Versions
            .Select(v => new MinecraftVersion(v.Id, v.Type == "snapshot"))
            .ToList();
    }

    public async Task<ResolvedServer> ResolveAsync(Server server)
    {
        var manifest = await _http.GetFromJsonAsync<VersionManifest>(ManifestUrl)
            ?? throw new HestiaException("Failed to fetch Minecraft version manifest.");

        var entry = manifest.Versions.Find(v => v.Id == server.Version)
            ?? throw new HestiaException($"Minecraft version '{server.Version}' not found.");

        var details = await _http.GetFromJsonAsync<VersionDetails>(entry.Url)
            ?? throw new HestiaException($"Failed to fetch details for version '{server.Version}'.");

        var artifact = details.Downloads.Server
            ?? throw new HestiaException($"Version '{server.Version}' has no server download.");

        var minJava = details.JavaVersion?.MajorVersion
            ?? throw new HestiaException($"Version '{server.Version}' has no Java version requirement.");

        return new ResolvedServer
        {
            Server = server,
            DownloadUrl = artifact.Url,
            Checksum = artifact.Sha1,
            MinJavaVersion = minJava,
        };
    }
}
