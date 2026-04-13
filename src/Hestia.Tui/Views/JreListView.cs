using Hestia.Tui.Screens;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Views;

internal static class JreListView
{
    public static IRenderable Render(MainViewModel vm)
    {
        var rows = vm.JreRows;
        var focused = vm.ActivePane == Pane.JRE;

        IRenderable body;
        if (rows.Count == 0)
        {
            body = new Markup("[dim]No runtimes found.[/]");
        }
        else
        {
            var table = new Table()
                .HideHeaders()
                .NoBorder()
                .AddColumn(new TableColumn(string.Empty).NoWrap())
                .Expand();
            foreach (var r in rows)
                table.AddRow(new Markup(Markup.Escape(r)));
            body = table;
        }

        return new Panel(body)
        {
            Header = new PanelHeader(focused ? "[bold]Java Runtimes[/]" : "Java Runtimes"),
            Border = focused ? BoxBorder.Double : BoxBorder.Rounded,
            Expand = true,
        };
    }
}
