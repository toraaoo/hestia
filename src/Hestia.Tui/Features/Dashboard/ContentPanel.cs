using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Tui.Features.Dashboard.Tabs;
using Hestia.Tui.Input;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard;

public sealed class ContentPanel(List<ITab> tabs)
{
    private int _activeTab;

    public void MoveLeft() => _activeTab = Math.Max(0, _activeTab - 1);
    public void MoveRight() => _activeTab = Math.Min(tabs.Count - 1, _activeTab + 1);

    public Task OnServerChangedAsync(Server? server, CancellationToken ct) =>
        tabs[_activeTab].OnServerChangedAsync(server, ct);

    public IRenderable Render(Server? server, bool focused)
    {
        var color = focused ? Color.Green : Color.Grey;
        var headerParts = tabs.Select((tab, i) =>
            i == _activeTab ? $"[bold {color}] {tab.Title} [/]" : $"[dim] {tab.Title} [/]");
        var tabBar = string.Join("[grey]·[/]", headerParts);

        return new Panel(tabs[_activeTab].Render(server))
            .Header(tabBar)
            .Border(BoxBorder.Rounded)
            .BorderColor(color)
            .Expand();
    }
}
