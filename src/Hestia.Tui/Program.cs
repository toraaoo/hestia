using Hestia.Core;
using Hestia.Core.Java;
using Hestia.Core.Utils;

namespace Hestia.Tui;

public static class Program
{
    private class ConsoleDownloadCallback : IProgressCallback
    {
        public void OnProgress(double progress)
        {
            Console.WriteLine($"\rDownload progress: {progress:P2}");
        }
    }

    private static async Task Main(string[] args)
    {
        var javaManager = new Manager();

        Console.WriteLine("Available Java versions:");
        var availableVersions = await javaManager.ListAvailableVersions();
        foreach (var version in availableVersions)
        {
            Console.WriteLine($"- {version.Major}.{version.Minor}.{version.Security}+{version.Build}");
        }

        Console.WriteLine("\nEnter the version you want to install (e.g., 17.0.2):");
        var versionToInstall = Console.ReadLine()?.Trim();

        if (!string.IsNullOrEmpty(versionToInstall))
        {
            Console.WriteLine($"Installing Java {versionToInstall}...");
            var callback = new ConsoleDownloadCallback();

            var installationPath = await javaManager.InstallAsync(versionToInstall, callback);
            Console.WriteLine($"Java {versionToInstall} installed at: {installationPath}");
        }
    }
}