using System.Runtime.InteropServices;

namespace Hestia.Core.Utils;

public class AppDataFileSystem
{
    private static string Root => RuntimeInfo.Current.Os == OSPlatform.Windows
        ? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "Hestia")
        : Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".hestia");

    public DownloadsDirectory Downloads { get; } = new(Root);
    public JavaDirectory Java { get; } = new(Root);
    public LogsDirectory Logs { get; } = new(Root);
    public ServersDirectory Servers { get; } = new(Root);

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

    public sealed class LogsDirectory(string root)
    {
        public string Dir { get; } = Path.Combine(root, "logs");

        public string GetFilePath(string filename)
        {
            Directory.CreateDirectory(Dir);
            return Path.Combine(Dir, filename);
        }
    }

    public sealed class ServersDirectory(string root)
    {
        public string Dir { get; } = Path.Combine(root, "servers");
        public string ServersJsonPath { get; } = Path.Combine(root, "servers.json");

        public string GetServerDir(Guid id) => Path.Combine(Dir, id.ToString());
        public string GetJarPath(Guid id) => Path.Combine(GetServerDir(id), "server.jar");
        public string GetLogsDir(Guid id) => Path.Combine(GetServerDir(id), "logs");
        public string GetPropertiesPath(Guid id) => Path.Combine(GetServerDir(id), "server.properties");

        public void EnsureServerDir(Guid id) => Directory.CreateDirectory(GetServerDir(id));

        public void DeleteServer(Guid id)
        {
            var path = GetServerDir(id);
            if (Directory.Exists(path))
                Directory.Delete(path, recursive: true);
        }
    }
}