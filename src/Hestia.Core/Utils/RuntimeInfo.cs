using System.Runtime.InteropServices;

namespace Hestia.Core.Utils;

public sealed class RuntimeInfo(OSPlatform os, Architecture arch)
{
    public OSPlatform Os { get; } = os;
    public Architecture Arch { get; } = arch;

    public static RuntimeInfo Current => new(GetCurrentOSPlatform(), GetCurrentArchitecture());

    private static OSPlatform GetCurrentOSPlatform()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return OSPlatform.Windows;
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            return OSPlatform.Linux;
        return RuntimeInformation.IsOSPlatform(OSPlatform.OSX)
            ? OSPlatform.OSX
            : throw new PlatformNotSupportedException("Unsupported operating system.");
    }

    private static Architecture GetCurrentArchitecture() => RuntimeInformation.ProcessArchitecture;
}