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

        var stack = new ScreenStack(keyMap);
        await stack.RunAsync(new DashboardScreen(), cancellationToken);

        return 0;
    }
}
