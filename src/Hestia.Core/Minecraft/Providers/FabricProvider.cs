using System.Net.Http.Json;
using Hestia.Core.Minecraft.Models;
using Hestia.Core.Minecraft.Providers.Dtos;

namespace Hestia.Core.Minecraft.Providers;

public sealed class FabricProvider : IProvider
{
    private const string MetaBase = "https://meta.fabricmc.net/v2";
    private const string MojangManifestUrl = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

    private readonly HttpClient _http = new();

    public ServerType Type => ServerType.Fabric;

    public async Task<List<MinecraftVersion>> GetVersionsAsync()
    {
        var versions = await _http.GetFromJsonAsync<List<FabricVersion>>($"{MetaBase}/versions/game")
                       ?? throw new HestiaException("Failed to fetch Fabric game versions.");

        return versions
            .Select(v => new MinecraftVersion(v.Version, !v.Stable))
            .ToList();
    }

    public async Task<ResolvedServer> ResolveAsync(Server server)
    {
        var loaders = await _http.GetFromJsonAsync<List<FabricVersion>>($"{MetaBase}/versions/loader")
                      ?? throw new HestiaException("Failed to fetch Fabric loader versions.");
        var installers = await _http.GetFromJsonAsync<List<FabricVersion>>($"{MetaBase}/versions/installer")
                         ?? throw new HestiaException("Failed to fetch Fabric installer versions.");

        var loader = loaders.FirstOrDefault(l => l.Stable)
                     ?? throw new HestiaException("No stable Fabric loader version available.");

        var installer = installers.FirstOrDefault(i => i.Stable)
                        ?? throw new HestiaException("No stable Fabric installer version available.");

        var minJava = await ResolveMinJavaVersionAsync(server.Version);

        return new ResolvedServer
        {
            Server = server,
            DownloadUrl =
                $"{MetaBase}/versions/loader/{server.Version}/{loader.Version}/{installer.Version}/server/jar",
            Checksum = string.Empty,
            MinJavaVersion = minJava,
        };
    }

    private async Task<int> ResolveMinJavaVersionAsync(string gameVersion)
    {
        var manifest = await _http.GetFromJsonAsync<VersionManifest>(MojangManifestUrl)
                       ?? throw new HestiaException("Failed to fetch Minecraft version manifest.");

        var entry = manifest.Versions.Find(v => v.Id == gameVersion)
                    ?? throw new HestiaException($"Minecraft version '{gameVersion}' not found.");

        var details = await _http.GetFromJsonAsync<VersionDetails>(entry.Url)
                      ?? throw new HestiaException($"Failed to fetch details for version '{gameVersion}'.");

        return details.JavaVersion?.MajorVersion
               ?? throw new HestiaException($"Version '{gameVersion}' has no Java version requirement.");
    }
}