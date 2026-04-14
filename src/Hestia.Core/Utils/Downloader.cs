using System.Security.Cryptography;

namespace Hestia.Core.Utils;

public class Downloader
{
    public interface IDownloadCallback
    {
        void OnStart() { }
        void OnProgress(double progress);
        void OnCompleted() { }
        void OnError(Exception ex) { }
    }

    private readonly HttpClient _httpClient = new();

    public async Task Download(
        string url,
        string destination,
        string? checksum = null,
        IDownloadCallback? callback = null
    )
    {
        var tempFile = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"));

        try
        {
            using var response = await _httpClient.GetAsync(
                url,
                HttpCompletionOption.ResponseHeadersRead
            );

            response.EnsureSuccessStatusCode();

            var totalBytes = response.Content.Headers.ContentLength;
            var canReportProgress = totalBytes.HasValue && callback != null;

            await using var contentStream = await response.Content.ReadAsStreamAsync();

            callback?.OnStart();

            await using (
                var fileStream = new FileStream(
                    tempFile,
                    FileMode.Create,
                    FileAccess.Write,
                    FileShare.None,
                    8192,
                    true
                )
            )
            {
                var buffer = new byte[8192];
                long totalReadBytes = 0;
                int readBytes;

                while ((readBytes = await contentStream.ReadAsync(buffer.AsMemory(0, buffer.Length))) > 0)
                {
                    await fileStream.WriteAsync(buffer.AsMemory(0, readBytes));
                    totalReadBytes += readBytes;

                    if (canReportProgress)
                    {
                        var progress = (double)totalReadBytes / totalBytes!.Value * 100;
                        callback?.OnProgress(Math.Min(progress, 100));
                    }
                }

                await fileStream.FlushAsync();
            }

            if (!string.IsNullOrEmpty(checksum))
            {
                using var sha256 = SHA256.Create();
                await using var stream = File.OpenRead(tempFile);

                var computedHash = await sha256.ComputeHashAsync(stream);
                var computedHashString = BitConverter
                    .ToString(computedHash)
                    .Replace("-", "")
                    .ToLowerInvariant();

                if (!computedHashString.Equals(checksum, StringComparison.OrdinalIgnoreCase))
                {
                    throw new DownloadException("Checksum validation failed.");
                }
            }

            if (File.Exists(destination))
            {
                File.Delete(destination);
            }

            File.Move(tempFile, destination);

            callback?.OnCompleted();
        }
        catch (Exception ex)
        {
            if (File.Exists(tempFile))
            {
                try
                {
                    File.Delete(tempFile);
                }
                catch
                {
                    // Ignore any exceptions during cleanup
                }
            }

            callback?.OnError(ex);
            throw;
        }
    }

    public async Task DownloadMultiple(
        IEnumerable<(string url, string destination, string? checksum)> downloads,
        IDownloadCallback? callback = null
    )
    {
        foreach (var (url, destination, checksum) in downloads)
        {
            try
            {
                await Download(url, destination, checksum, callback);
            }
            catch (Exception ex)
            {
                callback?.OnError(new DownloadException($"Failed to download {url}", ex));
            }
        }
    }
}