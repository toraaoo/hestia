using Hestia.Core.Utils;

namespace Hestia.Tui;

public static class Program
{
    private class ConsoleDownloadCallback : Downloader.IDownloadCallback
    {
        public void OnProgress(double progress)
        {
            Console.WriteLine($"Download progress: {progress:P2}");
        }
    }

    private static async Task Main(string[] args)
    {
        var downloader = new Downloader();
        const string url = "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.10%2B7/OpenJDK21U-jdk-sources_21.0.10_7.tar.gz";
        const string destination = "file.zip";


        await downloader.Download(
            url,
            destination,
            callback: new ConsoleDownloadCallback(),
            checksum: "a286b69953cdb56ab2dc74287e6ebaca8fad7d397a4b5f975b73d23eedeec251"
        );
        Console.WriteLine("Download completed successfully.");
    }
}