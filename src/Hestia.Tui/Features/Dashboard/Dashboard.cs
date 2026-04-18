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

public sealed class DashboardScreen : ScreenBase
{
    private enum Focus { ServerList, Content }

    private readonly Manager _manager;
    private readonly ServerListPanel _serverList;
    private readonly ContentPanel _content;
    private readonly INavigator _navigator;

    private Focus _focus = Focus.ServerList;
    private Layout? _layout;
    private string? _statusMessage;
    private bool _isWorking;
    private bool _needsReload;
    private CancellationTokenSource _statusCts = new();

    public DashboardScreen(Manager manager, INavigator navigator, Func<CreateServerScreen> createServerFactory)
    {
        _manager = manager;
        _navigator = navigator;

        _content = new ContentPanel([new LogsTab(manager), new StatusTab(manager)]);

        _serverList = new ServerListPanel(
            manager,
            onStart: id => _ = StartServerAsync(id),
            onStop: id => navigator.ShowModal(
                new ConfirmModal($"Stop '{_serverList.Selected?.Name}'?"),
                confirmed => { if (confirmed) _ = StopServerAsync(id); }),
            onDelete: id => navigator.ShowModal(
                new ConfirmModal($"Delete '{_serverList.Selected?.Name}'? This cannot be undone."),
                confirmed => { if (confirmed) _ = DeleteServerAsync(id); }),
            onNew: () => { _needsReload = true; navigator.Push(createServerFactory()); },
            onSelectionChanged: server =>
            {
                _statusMessage = null;
                _ = _content.OnServerChangedAsync(server, CancellationToken.None);
            }
        );
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
        if (action == InputAction.Quit) { _navigator.Quit(); return; }
        if (action == InputAction.Tab) { ToggleFocus(); return; }

        if (_focus == Focus.ServerList && !_isWorking)
            _serverList.OnInput(action);
        else if (_focus == Focus.Content)
            _content.OnInput(action);
    }

    public override bool OnRawKey(ConsoleKeyInfo key)
    {
        if (_focus == Focus.ServerList && !_isWorking)
            return _serverList.OnRawKey(key);
        if (_focus == Focus.Content)
            return _content.OnRawKey(key);
        return false;
    }

    private void ToggleFocus() =>
        _focus = _focus == Focus.ServerList ? Focus.Content : Focus.ServerList;

    private void SetTransientStatus(string message)
    {
        _statusCts.Cancel();
        _statusCts = new CancellationTokenSource();
        var ct = _statusCts.Token;
        _statusMessage = message;
        _ = Task.Delay(3000, ct).ContinueWith(_ => _statusMessage = null, ct);
    }

    private async Task StartServerAsync(Guid id)
    {
        _isWorking = true;
        _statusMessage = "Starting…";
        try
        {
            await _manager.StartAsync(id);
            _serverList.Reload();
        }
        catch (Exception ex)
        {
            SetTransientStatus($"Error: {ex.Message}");
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
            await _manager.StopAsync(id);
            _serverList.Reload();
            SetTransientStatus("Server stopped.");
        }
        catch (Exception ex)
        {
            SetTransientStatus($"Error: {ex.Message}");
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
            await _manager.DeleteAsync(id);
            _serverList.Reload();
            SetTransientStatus("Deleted.");
        }
        catch (Exception ex)
        {
            SetTransientStatus($"Error: {ex.Message}");
        }
        finally
        {
            _isWorking = false;
        }
    }

}
