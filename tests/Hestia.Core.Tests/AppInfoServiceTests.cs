using System;
using System.IO;
using System.Reflection;
using Hestia.Core;
using Xunit;

namespace Hestia.Core.Tests;

public sealed class AppInfoServiceTests
{
    [Fact]
    public void GetAppInfo_Invariants_Hold()
    {
        var hostAssembly = typeof(AppInfoServiceTests).Assembly;
        var info = new AppInfoService(hostAssembly).GetAppInfo();

        Assert.False(string.IsNullOrWhiteSpace(info.Version));
        Assert.False(string.IsNullOrWhiteSpace(info.AppDataDirectory));
        Assert.True(Path.IsPathRooted(info.AppDataDirectory));

        if (OperatingSystem.IsWindows())
        {
            var appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
            Assert.StartsWith(appData, info.AppDataDirectory, StringComparison.OrdinalIgnoreCase);
            Assert.EndsWith(Path.Combine("Hestia"), info.AppDataDirectory, StringComparison.OrdinalIgnoreCase);
        }
        else if (OperatingSystem.IsLinux())
        {
            var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            Assert.StartsWith(home, info.AppDataDirectory, StringComparison.Ordinal);
            Assert.EndsWith(Path.Combine(".hestia"), info.AppDataDirectory, StringComparison.Ordinal);
        }
    }
}
