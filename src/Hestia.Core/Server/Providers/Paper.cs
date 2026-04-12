using System.Net.Http.Json;
using System.Text.Json.Serialization;
using Hestia.Core.Abstractions;

namespace Hestia.Core.Server.Providers;

public sealed class Paper : IServerProvider
{
    private const string BaseUrl = "https://api.papermc.io/v2/projects/paper";

    private readonly HttpClient _http;

    public Paper(HttpClient http)
    {
        _http = http;
    }

    public ServerType ServerType => ServerType.Paper;

    public async ValueTask<IReadOnlyList<string>> GetAvailableVersionsAsync(CancellationToken ct = default)
    {
        var project = await _http.GetFromJsonAsync<Project>(BaseUrl, ct).ConfigureAwait(false)
                      ?? throw new InvalidOperationException("Failed to fetch Paper project metadata.");

        return project.Versions.ToList().AsReadOnly();
    }

    public async ValueTask DownloadServerJarAsync(
        string minecraftVersion,
        string destPath,
        IProgress<double>? progress = null,
        CancellationToken ct = default)
    {
        var versionInfo = await _http
            .GetFromJsonAsync<VersionInfo>($"{BaseUrl}/versions/{minecraftVersion}", ct)
            .ConfigureAwait(false)
            ?? throw new InvalidOperationException($"Failed to fetch Paper builds for '{minecraftVersion}'.");

        if (versionInfo.Builds.Count == 0)
            throw new InvalidOperationException($"No Paper builds available for '{minecraftVersion}'.");

        var build = versionInfo.Builds.Max();

        var buildInfo = await _http
            .GetFromJsonAsync<BuildInfo>($"{BaseUrl}/versions/{minecraftVersion}/builds/{build}", ct)
            .ConfigureAwait(false)
            ?? throw new InvalidOperationException(
                $"Failed to fetch Paper build metadata for '{minecraftVersion}' build {build}.");

        var app = buildInfo.Downloads.Application
                  ?? throw new InvalidOperationException(
                      $"Paper build metadata missing application download for '{minecraftVersion}' build {build}.");

        var url = $"{BaseUrl}/versions/{minecraftVersion}/builds/{build}/downloads/{app.Name}";
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

    private sealed record Project(
        [property: JsonPropertyName("versions")] List<string> Versions);

    private sealed record VersionInfo(
        [property: JsonPropertyName("builds")] List<int> Builds);

    private sealed record BuildInfo(
        [property: JsonPropertyName("downloads")] Downloads Downloads);

    private sealed record Downloads(
        [property: JsonPropertyName("application")] DownloadApplication? Application);

    private sealed record DownloadApplication(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("sha256")] string Sha256);
}
