using System.IO.Compression;
using Hestia.Core.Utils;
using ICSharpCode.SharpZipLib.Tar;

namespace Hestia.Core.Java;

public class ResolvedJava
{
    public string Version { get; init; } = "";
    public string DownloadUrl { get; init; } = "";
    public string Checksum { get; init; } = "";
    public string ChecksumType { get; init; } = "sha256";
    public long SizeBytes { get; init; }

    public string Os { get; init; } = "";
    public string Arch { get; init; } = "";

    private readonly Downloader _downloader = new();
    private readonly AppDataFileSystem _fileSystem = new();

    private string DownloadFilename => Os == "windows"
        ? $"jdk-{Version}.zip"
        : $"jdk-{Version}.tar.gz";

    public async Task<string> DownloadAndInstall(IProgressCallback? callback = null)
    {
        var downloadPath = _fileSystem.Downloads.GetFilePath(DownloadFilename);

        await _downloader.Download(
            DownloadUrl,
            downloadPath,
            checksum: Checksum,
            callback: callback
        );

        var javaDir = _fileSystem.Java.Dir;
        Directory.CreateDirectory(javaDir);

        var installationPath = _fileSystem.Java.GetInstallationDir(Version);

        try
        {
            await Task.Run(() =>
            {
                if (Os == "windows")
                {
                    ZipFile.ExtractToDirectory(downloadPath, javaDir, overwriteFiles: true);
                }
                else
                {
                    using var fileStream = File.OpenRead(downloadPath);
                    using var gzipStream = new GZipStream(fileStream, CompressionMode.Decompress);
                    using var tarArchive = TarArchive.CreateInputTarArchive(gzipStream, System.Text.Encoding.UTF8);
                    tarArchive.ExtractContents(javaDir);
                }

                var extracted = Directory.GetDirectories(javaDir)
                    .Except([installationPath])
                    .SingleOrDefault()
                    ?? throw new InvalidOperationException("Could not find extracted JDK directory.");

                if (Directory.Exists(installationPath))
                    Directory.Delete(installationPath, recursive: true);

                Directory.Move(extracted, installationPath);
            });

            return installationPath;
        }
        finally
        {
            if (File.Exists(downloadPath))
            {
                try { File.Delete(downloadPath); }
                catch { /* ignore cleanup errors */ }
            }
        }
    }
}
