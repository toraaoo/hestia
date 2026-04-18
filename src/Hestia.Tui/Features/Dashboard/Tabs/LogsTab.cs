using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Tui.Input;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard.Tabs;

public sealed class LogsTab(Manager manager) : ITab
{
    public string Title => "Logs";

    public IRenderable Render(Server? server) => new Markup("[dim]  (coming soon)[/]");
}
