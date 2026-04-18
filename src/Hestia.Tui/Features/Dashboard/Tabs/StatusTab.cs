using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard.Tabs;

public sealed class StatusTab(Manager manager) : Tab
{
    public override string Title => "Status";

    public override IRenderable Render()
    {
        if (Server is null)
            return new Markup("[dim]  (no server selected)[/]");

        var status = manager.GetStatus(Server.Id);
        var statusMarkup = status switch
        {
            ServerStatus.Running  => "[green]Running[/]",
            ServerStatus.Starting => "[yellow]Starting[/]",
            ServerStatus.Crashed  => "[red]Crashed[/]",
            _                     => "[dim]Stopped[/]",
        };

        var grid = new Grid().AddColumn().AddColumn();
        grid.AddRow("[dim]Name[/]",    $"[bold]{Server.Name}[/]");
        grid.AddRow("[dim]Status[/]",  statusMarkup);
        grid.AddRow("[dim]Type[/]",    Server.Type.ToString());
        grid.AddRow("[dim]Version[/]", Server.Version);
        grid.AddRow("[dim]Host[/]",    $"{Server.Host}:{Server.Network.Port}");
        grid.AddRow("[dim]World[/]",   Server.World.Name);
        return grid;
    }
}
