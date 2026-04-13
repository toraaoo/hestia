using Hestia.Tui.Screens;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Views;

internal static class CommandView
{
    public static IRenderable Render(MainViewModel vm)
    {
        var focused = vm.ActivePane == Pane.Command;
        var content = focused
            ? new Markup($"[bold]>[/] {Markup.Escape(vm.InputBuffer)}[blink]█[/]")
            : new Markup("[dim]/ → command[/]");

        return new Panel(content)
        {
            Header = new PanelHeader(focused ? "[bold]Command[/]" : "Command"),
            Border = focused ? BoxBorder.Double : BoxBorder.Rounded,
            Expand = true,
            Padding = new Padding(1, 0, 1, 0)
        };
    }
}
