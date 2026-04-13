using Hestia.Tui.Screens;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Views;

internal static class LogsView
{
    public static IRenderable Render(MainViewModel vm)
    {
        var lines = vm.LogLines;
        var focused = vm.ActivePane == Pane.Logs;

        var logTable = new Table()
            .HideHeaders()
            .NoBorder()
            .AddColumn(new TableColumn(string.Empty).NoWrap())
            .Expand();

        if (lines.Count == 0)
        {
            logTable.AddRow(new Markup("[dim]No logs yet...[/]"));
        }
        else
        {
            var viewport = Math.Clamp(Console.WindowHeight - 14, 10, 200);
            var end = Math.Clamp(lines.Count - vm.LogScroll, 0, lines.Count);
            var start = Math.Max(0, end - viewport);

            if (vm.LogScroll > 0)
                logTable.AddRow(new Markup("[dim]... (PgDn to newest)[/]"));

            for (var i = start; i < end; i++)
                logTable.AddRow(new Markup(Markup.Escape(lines[i])));
        }

        var tabBar = vm.ActiveTab == Tab.Logs
            ? "[bold underline]Logs[/]  [dim]Info[/]"
            : "[dim]Logs[/]  [bold underline]Info[/]";

        const string HeaderSuffix = "[dim]←→:tab  f:follow  PgUp/PgDn:scroll[/]";

        return new Panel(logTable)
        {
            Header = new PanelHeader($"{tabBar}  {HeaderSuffix}"),
            Border = focused ? BoxBorder.Double : BoxBorder.Rounded,
            Expand = true,
        };
    }
}
