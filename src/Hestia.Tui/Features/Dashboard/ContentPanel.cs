using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Tui.Features.Dashboard.Tabs;
using Hestia.Tui.Input;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard;

public sealed class ContentPanel(List<ITab> tabs) : IPanel
{
    private int _activeTab;
    private Server? _currentServer;

    public void MoveLeft() => _activeTab = Math.Max(0, _activeTab - 1);
    public void MoveRight() => _activeTab = Math.Min(tabs.Count - 1, _activeTab + 1);

    public Task OnServerChangedAsync(Server? server, CancellationToken ct)
    {
        _currentServer = server;
        return tabs[_activeTab].OnServerChangedAsync(server, ct);
    }

    public void OnInput(InputAction action)
    {
        switch (action)
        {
            case InputAction.MoveLeft:
                MoveLeft();
                break;
            case InputAction.MoveRight:
                MoveRight();
                break;
            default:
                tabs[_activeTab].OnInput(action);
                break;
        }
    }

    public bool OnRawKey(ConsoleKeyInfo key) => tabs[_activeTab].OnRawKey(key);

    public IRenderable Render(bool focused)
    {
        var color = focused ? Color.Green : Color.Grey;
        var headerParts = tabs.Select((tab, i) =>
            i == _activeTab ? $"[bold {color}] {tab.Title} [/]" : $"[dim] {tab.Title} [/]");
        var tabBar = string.Join("[grey]·[/]", headerParts);

        return new Panel(tabs[_activeTab].Render(_currentServer))
            .Header(tabBar)
            .Border(BoxBorder.Rounded)
            .BorderColor(color)
            .Expand();
    }
}
