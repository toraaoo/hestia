using System.Diagnostics;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using Hestia.Core.Minecraft.Models;

namespace Hestia.Core.Minecraft;

internal sealed class Launcher
{
    public ServerInstance Launch(Server server, string javaExePath, string serverDir)
    {
        string[] args = [..server.JvmArgs, "-jar", "server.jar", "nogui"];

        var startInfo = new ProcessStartInfo
        {
            FileName = javaExePath,
            Arguments = string.Join(' ', args),
            WorkingDirectory = serverDir,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            RedirectStandardInput = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };

        var subject = new Subject<string>();
        var process = new Process { StartInfo = startInfo, EnableRaisingEvents = true };

        process.OutputDataReceived += (_, e) =>
        {
            if (e.Data is not null)
                subject.OnNext(e.Data);
        };

        process.ErrorDataReceived += (_, e) =>
        {
            if (e.Data is not null)
                subject.OnNext(e.Data);
        };

        process.Exited += (_, _) => subject.OnCompleted();

        process.Start();
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();

        return new ServerInstance(server, process, subject.AsObservable());
    }
}
