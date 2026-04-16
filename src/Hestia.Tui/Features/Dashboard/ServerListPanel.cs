using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
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
        var table = new Table()
            .Expand()
            .HideHeaders()
            .Border(TableBorder.None)
            .AddColumn(new TableColumn("Server"));

        var color = focused ? Color.Green : Color.Grey;

        if (_servers.Count == 0)
        {
            table.AddRow("[dim]  (no servers)[/]");
        }
        else
        {
            foreach (var (server, i) in _servers.Select((s, i) => (s, i)))
            {
                var active = i == _cursor;
                var dot = _manager is null ? "[dim]○[/]" : StatusDot(_manager.GetStatus(server.Id));
                var label = active ? $"[bold {color}] {dot} {server.Name}[/]" : $"   {dot} {server.Name}";
                table.AddRow(label);
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
        ServerStatus.Running  => "[green]●[/]",
        ServerStatus.Starting => "[blink yellow]●[/]",
        ServerStatus.Crashed  => "[blink red]●[/]",
        _                     => "[dim]○[/]",
    };
}