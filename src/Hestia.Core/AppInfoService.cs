using System;
using System.Reflection;

namespace Hestia.Core;

public sealed class AppInfoService
{
    private readonly Assembly _hostAssembly;

    public AppInfoService(Assembly hostAssembly)
    {
        _hostAssembly = hostAssembly ?? throw new ArgumentNullException(nameof(hostAssembly));
    }

    public AppInfo GetAppInfo()
    {
        return new AppInfo(
            Version: GetHostVersion(_hostAssembly),
            AppDataDirectory: AppDataPath.GetAppDataDirectory());
    }

    private static string GetHostVersion(Assembly hostAssembly)
    {
        var informational = hostAssembly
            .GetCustomAttribute<AssemblyInformationalVersionAttribute>()
            ?.InformationalVersion;

        if (!string.IsNullOrWhiteSpace(informational))
        {
            return informational;
        }

        var nameVersion = hostAssembly.GetName().Version?.ToString();
        if (!string.IsNullOrWhiteSpace(nameVersion))
        {
            return nameVersion;
        }

        return "0.0.0";
    }
}
