using System.Net.Http.Json;
using System.Text.Json.Serialization;
using Hestia.Core.Abstractions;

namespace Hestia.Core.Server.Providers;

public sealed class Fabric : IServerProvider
{
    private const string GameVersionsUrl = "https://meta.fabricmc.net/v2/versions/game";
    private const string LoaderForGameUrl = "https://meta.fabricmc.net/v2/versions/loader";
    private const string InstallerVersionsUrl = "https://meta.fabricmc.net/v2/versions/installer";

    private readonly HttpClient _http;

    public Fabric(HttpClient http)
    {
        _http = http;
    }

    public ServerType ServerType => ServerType.Fabric;

    public async ValueTask<IReadOnlyList<string>> GetAvailableVersionsAsync(CancellationToken ct = default)
    {
        var games = await _http.GetFromJsonAsync<List<GameVersion>>(GameVersionsUrl, ct).ConfigureAwait(false)
                    ?? throw new InvalidOperationException("Failed to fetch Fabric game versions.");

        return games
            .Where(v => v.Stable && v.Version.IndexOf(' ') < 0 && v.Version.IndexOf('_') < 0)
            .Select(v => v.Version)
            .ToList()
            .AsReadOnly();
    }

    public async ValueTask DownloadServerJarAsync(
        string minecraftVersion,
        string destPath,
        IProgress<double>? progress = null,
        CancellationToken ct = default)
    {
        var loaders = await _http
            .GetFromJsonAsync<List<LoaderEntry>>($"{LoaderForGameUrl}/{minecraftVersion}", ct)
            .ConfigureAwait(false)
            ?? throw new InvalidOperationException($"Failed to fetch Fabric loader versions for '{minecraftVersion}'.");

        var loader = loaders.FirstOrDefault(l => l.Loader.Stable) ?? loaders.FirstOrDefault();
        if (loader is null)
            throw new InvalidOperationException($"No Fabric loader available for '{minecraftVersion}'.");

        var installers = await _http
            .GetFromJsonAsync<List<InstallerEntry>>(InstallerVersionsUrl, ct)
            .ConfigureAwait(false)
            ?? throw new InvalidOperationException("Failed to fetch Fabric installer versions.");

        var installer = installers.FirstOrDefault(i => i.Stable) ?? installers.FirstOrDefault();
        if (installer is null)
            throw new InvalidOperationException("No Fabric installer version available.");

        var url = $"https://meta.fabricmc.net/v2/versions/loader/{minecraftVersion}/{loader.Loader.Version}/{installer.Version}/server/jar";
        await DownloadWithProgressAsync(url, destPath, progress, ct).ConfigureAwait(false);
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

    private sealed record GameVersion(
        [property: JsonPropertyName("version")] string Version,
        [property: JsonPropertyName("stable")] bool Stable);

    private sealed record LoaderEntry(
        [property: JsonPropertyName("loader")] LoaderVersion Loader);

    private sealed record LoaderVersion(
        [property: JsonPropertyName("version")] string Version,
        [property: JsonPropertyName("stable")] bool Stable);

    private sealed record InstallerEntry(
        [property: JsonPropertyName("version")] string Version,
        [property: JsonPropertyName("stable")] bool Stable);
}
