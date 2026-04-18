using Hestia.Core.Minecraft;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard.Tabs;

public sealed class LogsTab(Manager manager) : Tab
{
    public override string Title => "Logs";

    public override IRenderable Render() => new Markup("[dim]  (coming soon)[/]");
}