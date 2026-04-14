using Hestia.Core.Utils;

namespace Hestia.Core.Java;

public class Manager
{
    private readonly Resolver _resolver = new();
    private readonly AppDataFileSystem _fileSystem = new();

    public async Task<string> InstallAsync(string version, IProgressCallback? callback = null)
    {
        var resolved = await _resolver.ResolveAsync(version);
        return await resolved.DownloadAndInstall(callback);
    }

    public void Uninstall(string version) => _fileSystem.Java.DeleteInstallation(version);

    public bool IsInstalled(string version) => _fileSystem.Java.InstallationExists(version);

    public IEnumerable<string> ListInstalled() => _fileSystem.Java.ListInstallations();

    public async Task<IEnumerable<JavaVersion>> ListAvailableVersions(
        int limit = 50,
        int page = 0,
        string vendor = "adoptopenjdk"
    ) => await _resolver.ListAvailableAsync(
        limit,
        page,
        vendor
    );
}