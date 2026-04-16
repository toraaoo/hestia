using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard.Tabs;

public sealed class LogsTab : ITab
{
    public string Title => "Logs";

    public IRenderable Render(Server? server, Manager manager) => new Markup("[dim]  (coming soon)[/]");
}
