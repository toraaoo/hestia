using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Tui.Utils.Extensions;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard;

public sealed class ServerListPanel
{
    private List<Server> _servers = [];
    private Manager? _manager;
    private int _cursor;

    public Server? Selected => _servers.Count > 0 ? _servers[_cursor] : null;

    public void Load(Manager manager)
    {
        _manager = manager;
        _servers = manager.List().ToList();
    }

    public void MoveUp() => _cursor = Math.Max(0, _cursor - 1);
    public void MoveDown() => _cursor = Math.Min(_servers.Count - 1, _cursor + 1);

    public IRenderable Render(bool focused)
    {
        var terminalWidth = AnsiConsole.Console.Profile.Width;
        var sidePanelWidth = Math.Max(40 - 2, terminalWidth / 4);
        const int cursorAndStatusWidth = 3;
        var columnWidth = (sidePanelWidth - 2 - cursorAndStatusWidth * 2) / 3;

        var table = new Table()
            .AddColumn(new TableColumn("").NoWrap().Centered().Width(3)) // cursor
            .AddColumn(new TableColumn("").NoWrap().Centered().Width(3)) // status
            .AddColumn(new TableColumn("Name").Centered().Width(columnWidth))
            .AddColumn(new TableColumn("Type").Centered().Width(columnWidth))
            .AddColumn(new TableColumn("Version").Centered().Width(columnWidth))
            .HideHeaders()
            .Border(TableBorder.None)
            .Expand();

        var color = focused ? Color.Green : Color.Grey;

        if (_servers.Count == 0)
        {
            table.AddRow("", "[dim(no servers)[/]");
        }
        else
        {
            var truncateLength = columnWidth - 4;

            foreach (var (server, i) in _servers.Select((s, i) => (s, i)))
            {
                var active = i == _cursor;
                var dot = _manager is null ? "[dim]○[/]" : StatusDot(_manager.GetStatus(server.Id));

                var name = server.Name.Truncate(truncateLength);
                var type = server.Type.ToString().Truncate(truncateLength);
                var version = server.Version.Truncate(truncateLength);

                table.AddRow(
                    active ? $"[{color}] →[/]" : "  ",
                    active ? $"[{color}]{dot}[/]" : dot,
                    active ? $"[{color}]{name}[/]" : name,
                    active ? $"[{color}]{type}[/]" : type,
                    active ? $"[{color}]{version}[/]" : version
                );
            }
        }

        return new Panel(table)
            .Header("[bold] Servers [/]")
            .Border(BoxBorder.Rounded)
            .BorderColor(focused ? Color.Green : Color.Grey)
            .Expand();
    }

    private static string StatusDot(ServerStatus status) => status switch
    {
        ServerStatus.Running => "[green]●[/]",
        ServerStatus.Starting => "[blink yellow]●[/]",
        ServerStatus.Crashed => "[blink red]●[/]",
        _ => "[dim]○[/]",
    };
}