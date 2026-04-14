using System.Runtime.InteropServices;

namespace Hestia.Core.Utils;

public class AppDataFileSystem
{
    public AppDataFileSystem()
    {
        var root = RuntimeInfo.Current.Os == OSPlatform.Windows
            ? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "Hestia")
            : Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".hestia");

        Downloads = new DownloadsDirectory(root);
        Java = new JavaDirectory(root);
    }

    public DownloadsDirectory Downloads { get; }
    public JavaDirectory Java { get; }

    public sealed class DownloadsDirectory(string root)
    {
        public string Dir { get; } = Path.Combine(root, "downloads");

        public string GetFilePath(string filename)
        {
            Directory.CreateDirectory(Dir);
            return Path.Combine(Dir, filename);
        }
    }

    public sealed class JavaDirectory(string root)
    {
        public string Dir { get; } = Path.Combine(root, "java");

        public string GetInstallationDir(string version) =>
            Path.Combine(Dir, $"jdk-{version}");

        public bool InstallationExists(string version) =>
            Directory.Exists(Path.Combine(Dir, $"jdk-{version}"));

        public void DeleteInstallation(string version)
        {
            var path = Path.Combine(Dir, $"jdk-{version}");
            if (Directory.Exists(path))
                Directory.Delete(path, recursive: true);
        }

        public IEnumerable<string> ListInstallations()
        {
            if (!Directory.Exists(Dir))
                return [];

            return Directory.GetDirectories(Dir)
                .Select(Path.GetFileName)
                .Where(name => name?.StartsWith("jdk-") == true)
                .OrderByDescending(x => x)!;
        }
    }
}