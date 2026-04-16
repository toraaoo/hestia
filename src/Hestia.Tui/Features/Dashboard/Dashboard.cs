using Hestia.Core.Minecraft;
using Hestia.Tui.Features.Dashboard.Tabs;
using Hestia.Tui.Input;
using Hestia.Tui.Navigation;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard;

public sealed class DashboardScreen(Manager manager) : ScreenBase
{
    private enum Focus
    {
        ServerList,
        Content
    }

    private readonly ServerListPanel _serverList = new();
    private readonly ContentPanel _content = new([new LogsTab(), new StatusTab()]);
    private Focus _focus = Focus.ServerList;
    private Layout? _layout;

    public override async Task LoadAsync(CancellationToken ct)
    {
        _serverList.Load(manager);
        await _content.OnServerChangedAsync(_serverList.Selected, ct);
    }

    public override IRenderable Render()
    {
        if (_layout is null)
        {
            _layout = new Layout("Root")
                .SplitRows(
                    new Layout("Main"),
                    new Layout("Footer").Size(1)
                );

            _layout["Main"].SplitColumns(
                new Layout("Left").Ratio(25),
                new Layout("Content").Ratio(75)
            );
        }

        _layout["Left"].Update(_serverList.Render(_focus == Focus.ServerList));
        _layout["Content"].Update(_content.Render(_serverList.Selected, _focus == Focus.Content));
        _layout["Footer"].Update(
            new Markup("[dim] [b]Tab[/] switch panel · [b]↑↓[/] navigate · [b]←→[/] cycle tabs · [b]Q[/] quit[/]")
        );

        return _layout;
    }

    public override void OnInput(InputAction action)
    {
        switch (action)
        {
            case InputAction.Quit:
                ScreenContext.Host.Quit();
                return;

            case InputAction.Tab:
                _focus = _focus == Focus.ServerList ? Focus.Content : Focus.ServerList;
                return;
        }

        if (_focus == Focus.ServerList)
        {
            var prev = _serverList.Selected;

            if (action == InputAction.MoveUp) _serverList.MoveUp();
            else if (action == InputAction.MoveDown) _serverList.MoveDown();

            if (_serverList.Selected != prev)
                _ = _content.OnServerChangedAsync(_serverList.Selected, CancellationToken.None);
        }
        else
        {
            switch (action)
            {
                case InputAction.MoveLeft:
                    _content.MoveLeft();
                    break;
                case InputAction.MoveRight:
                    _content.MoveRight();
                    break;
            }
        }
    }
}