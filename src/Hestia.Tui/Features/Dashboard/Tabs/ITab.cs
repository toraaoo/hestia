using Hestia.Core.Minecraft.Models;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard.Tabs;

public interface ITab
{
    string Title { get; }
    IRenderable Render(Server? server);
    Task OnServerChangedAsync(Server? server, CancellationToken ct) => Task.CompletedTask;
}
