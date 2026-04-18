using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard.Tabs;

public sealed class StatusTab(Manager manager) : ITab
{
    public string Title => "Status";

    public IRenderable Render(Server? server)
    {
        if (server is null)
            return new Markup("[dim]  (no server selected)[/]");

        var status = manager.GetStatus(server.Id);
        var statusMarkup = status switch
        {
            ServerStatus.Running  => "[green]Running[/]",
            ServerStatus.Starting => "[yellow]Starting[/]",
            ServerStatus.Crashed  => "[red]Crashed[/]",
            _                     => "[dim]Stopped[/]",
        };

        var grid = new Grid().AddColumn().AddColumn();
        grid.AddRow("[dim]Name[/]",    $"[bold]{server.Name}[/]");
        grid.AddRow("[dim]Status[/]",  statusMarkup);
        grid.AddRow("[dim]Type[/]",    server.Type.ToString());
        grid.AddRow("[dim]Version[/]", server.Version);
        grid.AddRow("[dim]Host[/]",    $"{server.Host}:{server.Network.Port}");
        grid.AddRow("[dim]World[/]",   server.World.Name);
        return grid;
    }
}
