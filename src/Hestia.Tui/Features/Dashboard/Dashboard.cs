using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Tui.Features.CreateServer;
using Hestia.Tui.Features.Dashboard.Tabs;
using Hestia.Tui.Input;
using Hestia.Tui.Modals;
using Hestia.Tui.Navigation;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard;

public sealed class DashboardScreen(
    Manager manager,
    INavigator navigator,
    Func<CreateServerScreen> createServerFactory) : ScreenBase
{
    private enum Focus
    {
        ServerList,
        Content
    }

    private readonly ServerListPanel _serverList = new();

    private readonly ContentPanel _content = new(
        [
            new LogsTab(manager),
            new StatusTab(manager)
        ]
    );

    private Focus _focus = Focus.ServerList;
    private Layout? _layout;
    private string? _statusMessage;
    private bool _isWorking;
    private bool _needsReload;

    public override async Task LoadAsync(CancellationToken ct)
    {
        _serverList.Load(manager);
        await _content.OnServerChangedAsync(_serverList.Selected, ct);
    }

    public override IRenderable Render()
    {
        if (_needsReload)
        {
            _serverList.Reload(manager);
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
        _layout["Content"].Update(_content.Render(_serverList.Selected, _focus == Focus.Content));
        _layout["Footer"].Update(_statusMessage is not null
            ? new Markup($"[dim] {_statusMessage}[/]")
            : new Markup(
                "[dim] [b]Tab[/] panel · [b]↑↓[/] nav · [b]←→[/] tabs · [b]N[/] new · [b]Enter[/] start/stop · [b]D[/] delete · [b]Q[/] quit[/]")
        );

        return _layout;
    }

    public override void OnInput(InputAction action)
    {
        switch (action)
        {
            case InputAction.Quit:
                navigator.Quit();
                return;

            case InputAction.Tab:
                _focus = _focus == Focus.ServerList ? Focus.Content : Focus.ServerList;
                return;
        }

        if (_focus == Focus.ServerList)
        {
            if (_isWorking) return;

            var prev = _serverList.Selected;

            if (action == InputAction.MoveUp) _serverList.MoveUp();
            else if (action == InputAction.MoveDown) _serverList.MoveDown();

            if (_serverList.Selected != prev)
            {
                _statusMessage = null;
                _ = _content.OnServerChangedAsync(_serverList.Selected, CancellationToken.None);
            }

            if (action == InputAction.New)
            {
                _needsReload = true;
                navigator.Push(createServerFactory());
            }
            else if (action == InputAction.Confirm && _serverList.Selected is { } sel)
            {
                var status = manager.GetStatus(sel.Id);
                if (status is ServerStatus.Stopped or ServerStatus.Crashed)
                    _ = StartServerAsync(sel.Id);
                else if (status == ServerStatus.Running)
                    navigator.ShowModal(
                        new ConfirmModal($"Stop '{sel.Name}'?"),
                        confirmed =>
                        {
                            if (confirmed) _ = StopServerAsync(sel.Id);
                        });
            }
            else if (action == InputAction.Delete && _serverList.Selected is { } del)
            {
                navigator.ShowModal(
                    new ConfirmModal($"Delete '{del.Name}'? This cannot be undone."),
                    confirmed =>
                    {
                        if (confirmed) _ = DeleteServerAsync(del.Id);
                    }
                );
            }
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

    private async Task StartServerAsync(Guid id)
    {
        _isWorking = true;
        _statusMessage = "Starting…";
        try
        {
            await manager.StartAsync(id);
            _serverList.Reload(manager);
            _statusMessage = "Server started.";
        }
        catch (Exception ex)
        {
            _statusMessage = $"Error: {ex.Message}";
        }
        finally
        {
            _isWorking = false;
        }
    }

    private async Task StopServerAsync(Guid id)
    {
        _isWorking = true;
        _statusMessage = "Stopping…";
        try
        {
            await manager.StopAsync(id);
            _serverList.Reload(manager);
            _statusMessage = "Server stopped.";
        }
        catch (Exception ex)
        {
            _statusMessage = $"Error: {ex.Message}";
        }
        finally
        {
            _isWorking = false;
        }
    }

    private async Task DeleteServerAsync(Guid id)
    {
        _isWorking = true;
        _statusMessage = "Deleting…";
        try
        {
            await manager.DeleteAsync(id);
            _serverList.Reload(manager);
            _statusMessage = "Deleted.";
        }
        catch (Exception ex)
        {
            _statusMessage = $"Error: {ex.Message}";
        }
        finally
        {
            _isWorking = false;
        }
    }
}