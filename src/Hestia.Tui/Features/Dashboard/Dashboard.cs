using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Tui.Features.CreateServer;
using Hestia.Tui.Features.Dashboard.Tabs;
using Hestia.Tui.Input;
using Hestia.Tui.Navigation;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard;

public sealed class DashboardScreen : ScreenBase
{
    private enum Focus
    {
        ServerList,
        Content
    }

    private readonly ServerListPanel _serverList;
    private readonly ContentPanel _content;
    private readonly INavigator _navigator;

    private Focus _focus = Focus.ServerList;
    private Layout? _layout;
    private bool _needsReload;
    private string? _statusMessage;

    public DashboardScreen(Manager manager, INavigator navigator, Func<CreateServerScreen> createServerFactory)
    {
        _navigator = navigator;

        _content = new ContentPanel([new LogsTab(manager), new StatusTab(manager)]);

        _serverList = new ServerListPanel(manager, navigator);

        _serverList.StatusChanged += msg => _statusMessage = msg;
        _serverList.NewRequested += () =>
        {
            _needsReload = true;
            navigator.Push(createServerFactory());
        };
        _serverList.SelectionChanged += server =>
            _ = _content.OnServerChangedAsync(server, CancellationToken.None);
    }

    public override async Task LoadAsync(CancellationToken ct)
    {
        _serverList.Load();
        await _content.OnServerChangedAsync(_serverList.Selected, ct);
    }

    public override IRenderable Render()
    {
        if (_needsReload)
        {
            _serverList.Reload();
            _needsReload = false;
        }

        if (_layout is null)
        {
            _layout = new Layout("Root")
                .SplitRows(
                    new Layout("Main"),
                    new Layout("Footer").Size(1)
                );

            _layout["Main"].SplitColumns(
                new Layout("Left").Ratio(25).MinimumSize(40),
                new Layout("Content").Ratio(75)
            );
        }

        _layout["Left"].Update(_serverList.Render(_focus == Focus.ServerList));
        _layout["Content"].Update(_content.Render(_focus == Focus.Content));
        _layout["Footer"].Update(_statusMessage is not null
            ? new Markup($"[dim] {_statusMessage}[/]")
            : new Markup("[dim] [b]Tab[/] panel · [b]↑↓[/] nav · [b]←→[/] tabs · [b]N[/] new · [b]Enter[/] start/stop · [b]D[/] delete · [b]Q[/] quit[/]")
        );

        return _layout;
    }

    public override void OnInput(InputAction action)
    {
        if (action == InputAction.Quit)
        {
            _navigator.Quit();
            return;
        }

        if (action == InputAction.Tab)
        {
            ToggleFocus();
            return;
        }

        switch (_focus)
        {
            case Focus.ServerList when !_serverList.IsWorking:
                _serverList.OnInput(action);
                break;
            case Focus.Content:
                _content.OnInput(action);
                break;
            default:
                throw new ArgumentOutOfRangeException(nameof(action), action, null);
        }
    }

    public override bool OnRawKey(ConsoleKeyInfo key)
    {
        return _focus switch
        {
            Focus.ServerList when !_serverList.IsWorking => _serverList.OnRawKey(key),
            Focus.Content => _content.OnRawKey(key),
            _ => false
        };
    }

    private void ToggleFocus() =>
        _focus = _focus == Focus.ServerList ? Focus.Content : Focus.ServerList;
}
