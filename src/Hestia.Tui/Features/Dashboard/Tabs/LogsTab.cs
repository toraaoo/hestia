using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Tui.Input;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard.Tabs;

public sealed class LogsTab(Manager manager) : Tab
{
    private const int MaxBuffer = 1000;

    private readonly List<string> _lines = [];
    private readonly Lock _lock = new();
    private IDisposable? _subscription;
    private ServerInstance? _subscribedInstance;
    private string _commandInput = "";

    public override string Title => "Logs";

    public override async Task OnServerChangedAsync(Server? server, CancellationToken ct)
    {
        await base.OnServerChangedAsync(server, ct);

        _subscription?.Dispose();
        _subscription = null;

        lock (_lock)
        {
            _lines.Clear();
            _commandInput = "";
        }

        if (server is null)
            return;

        var logPath = Path.Combine(manager.GetServerLogsDir(server.Id), "latest.log");
        if (File.Exists(logPath))
        {
            var history = await File.ReadAllLinesAsync(logPath, ct);
            lock (_lock)
                _lines.AddRange(history.TakeLast(MaxBuffer));
        }

        _subscription = SubscribeToLogs(server);
    }

    public override IRenderable Render()
    {
        if (Server is null)
            return new Markup("[dim]  no server selected[/]");

        // Resubscribe when instance changes (e.g. server restart creates a new instance)
        var currentInstance = manager.GetInstance(Server.Id);
        if (!ReferenceEquals(currentInstance, _subscribedInstance))
        {
            _subscription?.Dispose();
            _subscription = SubscribeToLogs(Server);
        }


        var isRunning = manager.GetStatus(Server.Id) is ServerStatus.Running or ServerStatus.Starting;

        var windowWidth = AnsiConsole.Profile.Width;
        var windowHeight = AnsiConsole.Profile.Height;

        var leftWidth = Math.Max(40, (int)(windowWidth * 0.25));
        var columnWidth = Math.Max(20, windowWidth - leftWidth - 6);
        var logRows = Math.Max(2, windowHeight - 4) - 1;

        var selected = new List<string>();
        var rowsUsed = 0;
        lock (_lock)
        {
            for (var i = _lines.Count - 1; i >= 0; i--)
            {
                var line = _lines[i].Replace("\t", "    ");
                var lineRows = Math.Max(1, (line.Length + columnWidth - 1) / columnWidth);
                if (rowsUsed + lineRows > logRows) break;
                selected.Insert(0, line);
                rowsUsed += lineRows;
            }
        }

        var table = new Table()
            .HideHeaders()
            .Border(TableBorder.None)
            .Expand()
            .AddColumn(new TableColumn(""));

        var padCount = Math.Max(0, logRows - rowsUsed);
        for (var i = 0; i < padCount; i++)
            table.AddRow(new Markup(""));

        foreach (var line in selected)
            table.AddRow(new Markup(Markup.Escape(line)));

        var statusRow = isRunning
            ? new Markup($"[dim]>[/] {Markup.Escape(_commandInput)}[blink]▌[/]")
            : new Markup("[dim] ● server stopped[/]");

        table.AddRow(statusRow);

        return table;
    }

    public override bool OnRawKey(ConsoleKeyInfo key)
    {
        if (Server is null) return false;
        if (manager.GetStatus(Server.Id) is not (ServerStatus.Running or ServerStatus.Starting))
            return false;
        if (key.Key == ConsoleKey.Enter) return false;

        if (key.Key == ConsoleKey.Backspace)
        {
            if (_commandInput.Length > 0)
                _commandInput = _commandInput[..^1];
            return true;
        }

        if (key.KeyChar is < ' ' or '\0') return false;

        _commandInput += key.KeyChar;
        return true;
    }

    public override void OnInput(InputAction action)
    {
        if (action != InputAction.Confirm || _commandInput.Length == 0 || Server is null)
            return;
        if (manager.GetStatus(Server.Id) is not (ServerStatus.Running or ServerStatus.Starting))
            return;

        var cmd = _commandInput;
        _commandInput = "";

        if (cmd.Equals("stop", StringComparison.OrdinalIgnoreCase))
            _ = manager.StopAsync(Server.Id);
        else
            _ = manager.GetInstance(Server.Id)?.SendUserCommandAsync(cmd);
    }

    private IDisposable? SubscribeToLogs(Server server)
    {
        var instance = manager.GetInstance(server.Id);
        _subscribedInstance = instance;

        return instance?.Output.Subscribe(line => AppendLine(line.Replace("\t", "    ")));
    }

    private void AppendLine(string line)
    {
        lock (_lock)
        {
            _lines.Add(line);
            if (_lines.Count > MaxBuffer)
                _lines.RemoveAt(0);
        }
    }
}