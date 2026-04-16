using Hestia.Core.Minecraft;
using Hestia.Core.Utils;
using Hestia.Tui.Features.Dashboard;
using Hestia.Tui.Input;
using Hestia.Tui.Navigation;
using Spectre.Console.Cli;

namespace Hestia.Tui;

public class App : AsyncCommand
{
    protected override async Task<int> ExecuteAsync(CommandContext context, CancellationToken cancellationToken)
    {
        var keyMap = new KeyMap();
        await keyMap.LoadAsync();

        var fs = new AppDataFileSystem();
        var javaManager = new Core.Java.Manager();
        var manager = new Manager(javaManager, fs);

        var stack = new ScreenStack(keyMap);
        await stack.RunAsync(new DashboardScreen(manager), cancellationToken);

        return 0;
    }
}
