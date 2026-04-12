using System.Net.Http.Json;
using System.Text.Json.Serialization;
using Hestia.Core.Abstractions;

namespace Hestia.Core.Server.Providers;

public sealed class Vanilla : IServerProvider
{
    private const string ManifestUrl =
        "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

    private readonly HttpClient _http;

    public Vanilla(HttpClient http)
    {
        _http = http;
    }

    public ServerType ServerType => ServerType.Vanilla;

    public async ValueTask<IReadOnlyList<string>> GetAvailableVersionsAsync(
        CancellationToken ct = default)
    {
        var manifest = await FetchManifestAsync(ct).ConfigureAwait(false);
        return manifest.Versions
            .Where(v => v.Type == "release")
            .Select(v => v.Id)
            .ToList()
            .AsReadOnly();
    }

    public async ValueTask DownloadServerJarAsync(
        string minecraftVersion,
        string destPath,
        IProgress<double>? progress = null,
        CancellationToken ct = default)
    {
        var manifest = await FetchManifestAsync(ct).ConfigureAwait(false);

        var entry = manifest.Versions.FirstOrDefault(v => v.Id == minecraftVersion)
            ?? throw new InvalidOperationException(
                $"Minecraft version '{minecraftVersion}' not found in Mojang manifest.");

        var versionMeta = await _http
            .GetFromJsonAsync<VersionMeta>(entry.Url, ct)
            .ConfigureAwait(false)
            ?? throw new InvalidOperationException(
                $"Failed to fetch version metadata for '{minecraftVersion}'.");

        var serverDownload = versionMeta.Downloads.Server
            ?? throw new InvalidOperationException(
                $"No server download available for Minecraft '{minecraftVersion}'.");

        await DownloadWithProgressAsync(serverDownload.Url, destPath, progress, ct)
            .ConfigureAwait(false);
    }

    private async Task<VersionManifest> FetchManifestAsync(CancellationToken ct)
    {
        return await _http
            .GetFromJsonAsync<VersionManifest>(ManifestUrl, ct)
            .ConfigureAwait(false)
            ?? throw new InvalidOperationException("Failed to fetch Mojang version manifest.");
    }

    private async Task DownloadWithProgressAsync(
        string url,
        string destPath,
        IProgress<double>? progress,
        CancellationToken ct)
    {
        using var response = await _http
            .GetAsync(url, HttpCompletionOption.ResponseHeadersRead, ct)
            .ConfigureAwait(false);

        response.EnsureSuccessStatusCode();

        var totalBytes = response.Content.Headers.ContentLength;
        var dir = Path.GetDirectoryName(destPath);
        if (!string.IsNullOrEmpty(dir))
            Directory.CreateDirectory(dir);

        await using var source = await response.Content.ReadAsStreamAsync(ct).ConfigureAwait(false);
        await using var dest = new FileStream(destPath, FileMode.Create, FileAccess.Write,
            FileShare.None, bufferSize: 81920, useAsync: true);

        var buffer = new byte[81920];
        long downloaded = 0;
        int bytesRead;

        while ((bytesRead = await source.ReadAsync(buffer, ct).ConfigureAwait(false)) > 0)
        {
            await dest.WriteAsync(buffer.AsMemory(0, bytesRead), ct).ConfigureAwait(false);
            downloaded += bytesRead;

            if (totalBytes.HasValue)
                progress?.Report((double)downloaded / totalBytes.Value);
        }

        progress?.Report(1.0);
    }

    private sealed record VersionManifest(
        [property: JsonPropertyName("versions")] List<VersionEntry> Versions);

    private sealed record VersionEntry(
        [property: JsonPropertyName("id")] string Id,
        [property: JsonPropertyName("type")] string Type,
        [property: JsonPropertyName("url")] string Url);

    private sealed record VersionMeta(
        [property: JsonPropertyName("downloads")] VersionDownloads Downloads);

    private sealed record VersionDownloads(
        [property: JsonPropertyName("server")] DownloadInfo? Server);

    private sealed record DownloadInfo(
        [property: JsonPropertyName("url")] string Url,
        [property: JsonPropertyName("sha1")] string Sha1,
        [property: JsonPropertyName("size")] long Size);
}
