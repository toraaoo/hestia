using Hestia.Tui.Formatting;
using Hestia.Tui.Screens;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Views;

internal static class ServerListView
{
    public static IRenderable Render(MainViewModel vm)
    {
        var servers = vm.Servers;
        var focused = vm.ActivePane == Pane.Servers;

        IRenderable body;
        if (servers.Count == 0)
        {
            body = new Markup("[dim]No servers. Press [bold]c[/] to create one.[/]");
        }
        else
        {
            var table = new Table()
                .HideHeaders()
                .NoBorder()
                .AddColumn(new TableColumn(string.Empty).NoWrap())
                .Expand();

            for (var i = 0; i < servers.Count; i++)
            {
                var row = Markup.Escape(RowFormatters.ServerRow(servers[i]));
                table.AddRow(i == vm.ServerCursor && focused
                    ? new Markup($"[bold reverse] {row} [/]")
                    : new Markup($" {row}"));
            }

            body = table;
        }

        return new Panel(body)
        {
            Header = new PanelHeader(focused ? "[bold]Servers[/]" : "Servers"),
            Border = focused ? BoxBorder.Double : BoxBorder.Rounded,
            Expand = true,
        };
    }
}
