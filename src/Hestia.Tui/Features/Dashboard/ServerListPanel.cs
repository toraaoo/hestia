using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Tui.Input;
using Hestia.Tui.Modals;
using Hestia.Tui.Navigation;
using Hestia.Tui.Utils.Extensions;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard;

public sealed class ServerListPanel(Manager manager, INavigator navigator) : IPanel
{
    private List<Server> _servers = [];
    private int _cursor;
    private CancellationTokenSource _statusCts = new();

    public Server? Selected => _servers.Count > 0 ? _servers[_cursor] : null;
    public bool IsWorking { get; private set; }

    public event Action<string?>? StatusChanged;
    public event Action? NewRequested;
    public event Action<Server?>? SelectionChanged;

    public void Load() => Reload();

    public void Reload()
    {
        var updated = manager.List().ToList();
        _cursor = Math.Clamp(_cursor, 0, Math.Max(0, updated.Count - 1));
        _servers = updated;
    }

    public bool OnRawKey(ConsoleKeyInfo key) => false;

    public void OnInput(InputAction action)
    {
        var prev = Selected;

        switch (action)
        {
            case InputAction.MoveUp:
                _cursor = Math.Max(0, _cursor - 1);
                break;
            case InputAction.MoveDown:
                _cursor = Math.Min(_servers.Count - 1, _cursor + 1);
                break;
        }

        if (Selected != prev)
            SelectionChanged?.Invoke(Selected);

        switch (action)
        {
            case InputAction.New:
                NewRequested?.Invoke();
                break;
            case InputAction.Confirm when Selected is { } sel:
            {
                var status = manager.GetStatus(sel.Id);
                switch (status)
                {
                    case ServerStatus.Stopped or ServerStatus.Crashed:
                        _ = StartServerAsync(sel.Id);
                        break;
                    case ServerStatus.Running:
                        navigator.ShowModal(
                            new ConfirmModal($"Stop '{Selected?.Name}'?"),
                            confirmed =>
                            {
                                if (confirmed) _ = StopServerAsync(sel.Id);
                            });
                        break;
                }

                break;
            }
            case InputAction.Delete when Selected is { } del:
                navigator.ShowModal(
                    new ConfirmModal($"Delete '{Selected?.Name}'? This cannot be undone."),
                    confirmed =>
                    {
                        if (confirmed) _ = DeleteServerAsync(del.Id);
                    });
                break;
        }
    }

    public IRenderable Render(bool focused)
    {
        var terminalWidth = AnsiConsole.Console.Profile.Width;
        var sidePanelWidth = Math.Max(40 - 2, terminalWidth / 4);
        const int cursorAndStatusWidth = 3;
        var columnWidth = (sidePanelWidth - 2 - cursorAndStatusWidth * 2) / 3;

        var table = new Table()
            .HideHeaders()
            .Border(TableBorder.None)
            .Expand();

        var color = focused ? Color.Green : Color.Grey;

        if (_servers.Count == 0)
        {
            table
                .AddColumn(new TableColumn("").NoWrap().Width(sidePanelWidth - 2))
                .AddRow("[dim] (no servers)[/]");
        }
        else
        {
            table.AddColumn(new TableColumn("").NoWrap().Centered().Width(3)) // cursor
                .AddColumn(new TableColumn("").NoWrap().Centered().Width(3)) // status
                .AddColumn(new TableColumn("Name").Centered().Width(columnWidth))
                .AddColumn(new TableColumn("Type").Centered().Width(columnWidth))
                .AddColumn(new TableColumn("Version").Centered().Width(columnWidth));

            var truncateLength = columnWidth - 4;

            foreach (var (server, i) in _servers.Select((s, i) => (s, i)))
            {
                var active = i == _cursor;
                var dot = StatusDot(manager.GetStatus(server.Id));

                var name = server.Name.Truncate(truncateLength);
                var type = server.Type.ToString().Truncate(truncateLength);
                var version = server.Version.Truncate(truncateLength);

                table.AddRow(
                    active ? $"[{color}] →[/]" : "  ",
                    active ? $"[{color}]{dot}[/]" : dot,
                    active ? $"[{color}]{name}[/]" : name,
                    active ? $"[{color}]{type}[/]" : type,
                    active ? $"[{color}]{version}[/]" : version
                );
            }
        }

        return new Panel(table)
            .Header("[bold] Servers [/]")
            .Border(BoxBorder.Rounded)
            .BorderColor(focused ? Color.Green : Color.Grey)
            .Expand();
    }

    private void SetTransientStatus(string message)
    {
        _statusCts.Cancel();
        _statusCts = new CancellationTokenSource();
        var ct = _statusCts.Token;
        StatusChanged?.Invoke(message);
        _ = Task.Delay(3000, ct).ContinueWith(_ => StatusChanged?.Invoke(null), ct);
    }

    private async Task StartServerAsync(Guid id)
    {
        IsWorking = true;
        SetTransientStatus("Starting…");
        try
        {
            await manager.StartAsync(id);
            Reload();
            SetTransientStatus("Server started.");
        }
        catch (Exception ex)
        {
            SetTransientStatus($"Error: {ex.Message}");
        }
        finally
        {
            IsWorking = false;
        }
    }

    private async Task StopServerAsync(Guid id)
    {
        IsWorking = true;
        SetTransientStatus("Stopping…");
        try
        {
            await manager.StopAsync(id);
            Reload();
            SetTransientStatus("Server stopped.");
        }
        catch (Exception ex)
        {
            SetTransientStatus($"Error: {ex.Message}");
        }
        finally
        {
            IsWorking = false;
        }
    }

    private async Task DeleteServerAsync(Guid id)
    {
        IsWorking = true;
        SetTransientStatus("Deleting…");
        try
        {
            await manager.DeleteAsync(id);
            Reload();
            SetTransientStatus("Deleted.");
        }
        catch (Exception ex)
        {
            SetTransientStatus($"Error: {ex.Message}");
        }
        finally
        {
            IsWorking = false;
        }
    }

    private static string StatusDot(ServerStatus status) => status switch
    {
        ServerStatus.Running => "[green]●[/]",
        ServerStatus.Starting => "[blink yellow]●[/]",
        ServerStatus.Crashed => "[blink red]●[/]",
        _ => "[dim]○[/]",
    };
}
