using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard;

public sealed class ServerListPanel
{
    private List<Server> _servers = [];
    private int _cursor;

    public Server? Selected => _servers.Count > 0 ? _servers[_cursor] : null;

    public void Load(Manager manager) => _servers = manager.List().ToList();

    public void MoveUp() => _cursor = Math.Max(0, _cursor - 1);
    public void MoveDown() => _cursor = Math.Min(_servers.Count - 1, _cursor + 1);

    public IRenderable Render(bool focused)
    {
        var table = new Table()
            .Expand()
            .HideHeaders()
            .Border(TableBorder.None)
            .AddColumn(new TableColumn("Server"));

        var color = focused ? Color.Yellow : Color.Grey;

        if (_servers.Count == 0)
        {
            table.AddRow("[dim]  (no servers)[/]");
        }
        else
        {
            foreach (var (server, i) in _servers.Select((s, i) => (s, i)))
            {
                var active = i == _cursor;
                table.AddRow(active ? $"[bold {color}]→  {server.Name}[/]" : $"   {server.Name}");
            }
        }

        return new Panel(table)
            .Header("[bold]Servers[/]")
            .Border(BoxBorder.Rounded)
            .BorderColor(focused ? Color.Yellow : Color.Grey)
            .Expand();
    }
}