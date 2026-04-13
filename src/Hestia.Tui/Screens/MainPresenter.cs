using Hestia.Core;
using Hestia.Core.Abstractions;
using Hestia.Core.Server;
using Hestia.Tui.Input;
using Hestia.Tui.Screens.Modals;
using Hestia.Tui.Services;
using Hestia.Tui.ViewModels;
using static Hestia.Tui.Utilities.ServerUtils;

namespace Hestia.Tui.Screens;

internal sealed class MainPresenter : IAsyncDisposable
{
    private readonly IHestiaService _service;
    private readonly AppInfo _appInfo;
    private readonly string _stamp;
    private readonly UiDispatcher _ui;
    private readonly KeyMap _keyMap;

    private ServerSessionVm? _session;

    public ServerListVm ServerListVm { get; }
    public JreListVm JreListVm { get; }
    public CommandVm CommandVm { get; }

    public Guid? SelectedServerId { get; private set; }
    public int ServerCursor { get; set; }
    public Pane ActivePane { get; set; } = Pane.Servers;
    public Tab ActiveTab { get; set; } = Tab.Logs;
    public int LogScroll { get; set; }
    public bool LogFollow { get; set; } = true;
    public string InputBuffer { get; set; } = string.Empty;
    public string StatusMsg { get; private set; } = string.Empty;
    public bool StatusIsError { get; private set; }
    public bool ShowRconPassword { get; set; }

    private readonly Queue<ModalRequest> _modalQueue = new();
    private ModalRequest? _activeModal;
    private CancellationTokenSource? _appCts;
    private Task? _pollTask;

    public MainPresenter(
        IHestiaService service,
        AppInfo appInfo,
        string stamp,
        UiDispatcher ui,
        KeyMap keyMap,
        ServerListVm serverListVm,
        JreListVm jreListVm,
        CommandVm commandVm)
    {
        _service = service;
        _appInfo = appInfo;
        _stamp = stamp;
        _ui = ui;
        _keyMap = keyMap;
        ServerListVm = serverListVm;
        JreListVm = jreListVm;
        CommandVm = commandVm;
    }

    public UiDispatcher Ui => _ui;

    public MainViewModel Snapshot()
    {
        var lines = _session?.LogBuffer.Snapshot() ?? [];

        if (LogFollow) LogScroll = 0;
        LogScroll = Math.Clamp(LogScroll, 0, Math.Max(0, lines.Count - 1));

        var selected = SelectedServerId is { } id
            ? ServerListVm.Servers.FirstOrDefault(x => x.Id == id)
            : null;

        return new MainViewModel(
            Servers: ServerListVm.Servers,
            JreRows: JreListVm.Rows,
            SelectedServerId: SelectedServerId,
            SelectedServer: selected,
            ServerCursor: ServerCursor,
            ActivePane: ActivePane,
            ActiveTab: ActiveTab,
            LogScroll: LogScroll,
            LogFollow: LogFollow,
            InputBuffer: InputBuffer,
            StatusMsg: StatusMsg,
            StatusIsError: StatusIsError,
            ShowRconPassword: ShowRconPassword,
            LogLines: lines,
            AppVersion: _appInfo.Version,
            Stamp: _stamp,
            LatestStatus: _session?.LatestStatus);
    }

    public void SetStatus(string msg, bool isError = false)
    {
        StatusMsg = msg;
        StatusIsError = isError;
    }

    public void AppendRconOutputToLogs(string line)
    {
        if (_session is null) return;
        _session.LogBuffer.Add(line);
    }

    public async Task SelectServerAsync(Guid id)
    {
        if (_session is not null)
        {
            await _session.DisposeAsync();
            _session = null;
        }

        SelectedServerId = id;
        ActivePane = Pane.Logs;
        ActiveTab = Tab.Logs;

        var name = ServerListVm.Servers.FirstOrDefault(s => s.Id == id)?.Name ?? id.ToString()[..8];
        SetStatus($"Selected: {name}");

        _session = new ServerSessionVm(_service, _ui, id);
        _session.Start();
    }

    public async Task DeselectAsync()
    {
        if (_session is not null)
        {
            await _session.DisposeAsync();
            _session = null;
        }

        SelectedServerId = null;
        ActivePane = Pane.Servers;
        SetStatus(string.Empty);
    }

    public async ValueTask DisposeAsync()
    {
        _appCts?.Cancel();

        if (_pollTask is not null)
        {
            try { await _pollTask; } catch { }
            _pollTask = null;
        }

        if (_session is not null)
        {
            await _session.DisposeAsync();
            _session = null;
        }
    }

    public ModalRequest? TryDequeueModal()
    {
        lock (_modalQueue)
        {
            if (_modalQueue.Count == 0)
                return null;

            _activeModal = _modalQueue.Dequeue();
            return _activeModal;
        }
    }

    public bool HasPendingModal
    {
        get
        {
            lock (_modalQueue)
            {
                return _modalQueue.Count > 0;
            }
        }
    }

    private void EnqueueModal(ModalRequest req)
    {
        lock (_modalQueue)
        {
            _modalQueue.Enqueue(req);
        }
    }

    public void OnKey(ConsoleKeyInfo key)
    {
        var action = _keyMap.Translate(key);

        if (ActivePane == Pane.Command)
        {
            HandleCommandKey(key, action);
            return;
        }

        switch (action)
        {
            case InputAction.OpenCommand when SelectedServerId.HasValue:
                ActivePane = Pane.Command;
                return;
            case InputAction.TogglePassword when SelectedServerId.HasValue:
                ShowRconPassword = !ShowRconPassword;
                return;
            case InputAction.Escape when ActivePane is Pane.Logs or Pane.Info:
                ActivePane = Pane.Servers;
                return;
            case InputAction.Escape when ActivePane == Pane.Servers && SelectedServerId.HasValue:
                _ = DeselectAsync();
                return;
            case InputAction.CycleFocusNext:
                CycleFocus(1);
                return;
            case InputAction.CycleFocusPrev:
                CycleFocus(-1);
                return;
            case InputAction.TabLeft when ActivePane == Pane.Info:
                ActiveTab = Tab.Logs;
                ActivePane = Pane.Logs;
                return;
            case InputAction.TabLeft when ActivePane == Pane.Logs:
                ActivePane = Pane.Servers;
                return;
            case InputAction.TabRight when ActivePane == Pane.Servers && SelectedServerId.HasValue:
                ActivePane = Pane.Logs;
                ActiveTab = Tab.Logs;
                return;
            case InputAction.TabRight when ActivePane == Pane.Logs:
                ActiveTab = Tab.Info;
                ActivePane = Pane.Info;
                return;
            case InputAction.ToggleFollow when SelectedServerId.HasValue:
                LogFollow = !LogFollow;
                return;
            case InputAction.ServerCreate:
                _ = RequestCreateAsync();
                return;
        }

        if (ActivePane == Pane.Servers)
        {
            HandleServersKey(action);
        }

        if (ActivePane == Pane.Logs)
        {
            switch (action)
            {
                case InputAction.CursorUp:
                    ScrollLogs(+1);
                    return;
                case InputAction.CursorDown:
                    ScrollLogs(-1);
                    return;
                case InputAction.PageUp:
                    ScrollLogs(+5);
                    return;
                case InputAction.PageDown:
                    ScrollLogs(-5);
                    return;
            }
        }
    }

    private void HandleServersKey(InputAction? action)
    {
        switch (action)
        {
            case InputAction.CursorUp:
                ServerCursor = Math.Max(0, ServerCursor - 1);
                break;
            case InputAction.CursorDown:
                ServerCursor = Math.Min(ServerListVm.Servers.Count - 1, ServerCursor + 1);
                break;
            case InputAction.Confirm:
            {
                var s = ServerListVm.GetAt(ServerCursor);
                if (s is not null) _ = SelectServerAsync(s.Id);
                break;
            }
            case InputAction.ServerMenu:
            {
                var s = ServerListVm.GetAt(ServerCursor);
                if (s is not null)
                    EnqueueModal(new ServerMenuModalRequest(s.Id, s));
                break;
            }
        }
    }

    private void HandleCommandKey(ConsoleKeyInfo k, InputAction? action)
    {
        switch (action)
        {
            case InputAction.Confirm:
            {
                var cmd = InputBuffer;
                InputBuffer = string.Empty;
                if (!string.IsNullOrWhiteSpace(cmd) && SelectedServerId is { } id && _appCts is not null)
                {
                    LogFollow = true;
                    LogScroll = 0;
                    _ = CommandVm.SendAsync(id, cmd, _appCts.Token);
                }

                break;
            }
            case InputAction.Escape:
                InputBuffer = string.Empty;
                ActivePane = Pane.Logs;
                break;
            case InputAction.PageUp:
                ScrollLogs(+5);
                break;
            case InputAction.PageDown:
                ScrollLogs(-5);
                break;
            case InputAction.TextBackspace:
                if (InputBuffer.Length > 0)
                    InputBuffer = InputBuffer[..^1];
                break;
            case InputAction.CursorUp:
            {
                var cur = InputBuffer;
                CommandVm.HistoryUp(ref cur);
                InputBuffer = cur;
                break;
            }
            case InputAction.CursorDown:
            {
                var cur = InputBuffer;
                CommandVm.HistoryDown(ref cur);
                InputBuffer = cur;
                break;
            }
            default:
                if (!char.IsControl(k.KeyChar))
                    InputBuffer += k.KeyChar;
                break;
        }
    }

    private void ScrollLogs(int delta)
    {
        var lines = _session?.LogBuffer.Snapshot() ?? [];
        var viewport = Math.Clamp(Console.WindowHeight - 14, 10, 200);
        var maxScroll = Math.Max(0, lines.Count - viewport);

        if (delta > 0) LogFollow = false;
        LogScroll = Math.Clamp(LogScroll + delta, 0, maxScroll);
        if (LogScroll == 0) LogFollow = true;
    }

    private void CycleFocus(int dir)
    {
        Pane[] panes = SelectedServerId.HasValue
            ? [Pane.Servers, Pane.Logs, Pane.Info]
            : [Pane.Servers];

        var idx = Array.IndexOf(panes, ActivePane);
        idx = ((idx + dir) % panes.Length + panes.Length) % panes.Length;
        ActivePane = panes[idx];

        if (ActivePane == Pane.Logs) ActiveTab = Tab.Logs;
        if (ActivePane == Pane.Info) ActiveTab = Tab.Info;
    }

    public async Task StartAsync(CancellationToken ct)
    {
        _appCts = CancellationTokenSource.CreateLinkedTokenSource(ct);

        try
        {
            await Task.WhenAll(
                ServerListVm.RefreshAsync(_appCts.Token),
                JreListVm.RefreshAsync(_appCts.Token));
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            Ui.Post(() => SetStatus($"Load error: {ex.Message}", true));
        }

        _pollTask = Task.Run(PollServerListAsync, _appCts.Token);
    }

    private async Task PollServerListAsync()
    {
        if (_appCts is null) return;

        while (!_appCts.IsCancellationRequested)
        {
            try
            {
                await Task.Delay(2000, _appCts.Token);
                await ServerListVm.RefreshAsync(_appCts.Token);
                Ui.Post(() =>
                {
                    if (ServerListVm.Servers.Count > 0)
                        ServerCursor = Math.Min(ServerCursor, ServerListVm.Servers.Count - 1);
                });
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch
            {
            }
        }
    }

    public async Task HandleModalResultAsync(ModalResult result, CancellationToken ct)
    {
        try
        {
            switch (result)
            {
                case ServerMenuModalResult m:
                    if (m.Action is { } action)
                        await HandleServerMenuActionAsync(action, ct);
                    break;
                case DeleteModalResult d:
                    await HandleDeleteConfirmedAsync(d.Confirmed, ct);
                    break;
                case CreateModalResult c:
                    if (c.Form is not null)
                        _ = RunCreateAsync(c.Form, ct);
                    break;
                case ProgressModalResult:
                    break;
            }
        }
        finally
        {
            _activeModal = null;
        }
    }

    private Task HandleServerMenuActionAsync(InputAction action, CancellationToken ct)
    {
        var server = (_activeModal as ServerMenuModalRequest)?.Server;
        if (server is null)
            server = ServerListVm.GetAt(ServerCursor);
        if (server is null) return Task.CompletedTask;

        if (action == InputAction.ServerDelete)
        {
            EnqueueModal(new DeleteModalRequest(server.Id, server.Name));
            return Task.CompletedTask;
        }

        var task = action switch
        {
            InputAction.ServerStart => ServerListVm.StartAsync(server.Id, ct),
            InputAction.ServerStop => ServerListVm.StopAsync(server.Id, ct),
            InputAction.ServerRestart => ServerListVm.RestartAsync(server.Id, ct),
            _ => Task.CompletedTask,
        };
        var verb = action switch
        {
            InputAction.ServerStart => "Starting",
            InputAction.ServerStop => "Stopping",
            InputAction.ServerRestart => "Restarting",
            _ => ""
        };

        SetStatus($"{verb} {server.Name}…");

        _ = task.ContinueWith(t =>
        {
            if (t.IsFaulted)
            {
                Ui.Post(() => SetStatus($"Error: {t.Exception?.InnerException?.Message}", true));
                return;
            }

            if (action is InputAction.ServerStart or InputAction.ServerRestart)
                _ = SelectServerAsync(server.Id);
            else
                Ui.Post(() => SetStatus($"{server.Name}: done"));
        }, TaskScheduler.Default);

        return Task.CompletedTask;
    }

    private async Task HandleDeleteConfirmedAsync(bool confirmed, CancellationToken ct)
    {
        if (!confirmed) return;
        var req = _activeModal as DeleteModalRequest;
        if (req is null) return;

        SetStatus($"Deleting {req.ServerName}…");

        try
        {
            await ServerListVm.DeleteAsync(req.ServerId, ct);
            Ui.Post(() =>
            {
                SetStatus($"Deleted: {req.ServerName}");
                _ = DeselectAsync();
            });
        }
        catch (Exception ex)
        {
            Ui.Post(() => SetStatus($"Error: {ex.Message}", true));
        }
    }

    private async Task RequestCreateAsync()
    {
        if (_appCts is null) return;

        try
        {
            var initialType = ServerType.Vanilla;
            var types = new[] { ServerType.Vanilla, ServerType.Paper, ServerType.Fabric };
            var byType = new Dictionary<ServerType, IReadOnlyList<string>>();
            foreach (var t in types)
                byType[t] = await _service.GetAvailableVersionsAsync(t, _appCts.Token);

            var form = new ServerCreateForm(_appInfo.AppDataDirectory, byType[initialType]);
            form.Type = initialType;

            var existing = await _service.GetServersAsync(_appCts.Token);
            var used = new HashSet<int>(existing.SelectMany(s => new[] { s.Options.Port, s.RconOptions.Port }));
            form.ServerPort = FindNextFreePort(25565, used);
            used.Add(form.ServerPort);
            form.RconPort = FindNextFreePort(25575, used);

            EnqueueModal(new CreateModalRequest(form, byType));
        }
        catch (Exception ex)
        {
            Ui.Post(() => SetStatus($"Create setup error: {ex.Message}", true));
        }
    }

    private async Task RunCreateAsync(ServerCreateForm form, CancellationToken ct)
    {
        if (_appCts is null) return;

        var state = new ProgressState
        {
            ServerName = form.Name,
            Version = form.Version,
            Type = form.Type.ToString(),
            Progress = 0.0,
            StatusMsg = "Downloading server files...",
        };
        EnqueueModal(new ProgressModalRequest(state));

        try
        {
            var progress = new Progress<double>(v => state.Progress = Math.Clamp(v, 0.0, 1.0));

            var serverOpts = new ServerOptions(
                ServerDirectory: form.Directory,
                Port: form.ServerPort,
                MaxPlayers: form.MaxPlayers,
                MotD: form.MotD,
                ViewDistance: form.ViewDistance,
                OnlineMode: form.OnlineMode,
                Whitelist: form.Whitelist,
                LevelName: form.LevelName,
                Difficulty: form.Difficulty);

            var rconOpts = new RconOptions(
                Port: form.RconPort,
                Password: form.RconPassword,
                Enabled: form.RconEnabled,
                ConnectTimeoutSeconds: form.RconTimeoutSeconds);

            var flags = SplitJvmFlags(form.JvmAdditionalFlags);
            var jvmOpts = new JvmOptions(
                MinMemory: form.JvmMinMemory,
                MaxMemory: form.JvmMaxMemory,
                AdditionalFlags: flags.Count == 0 ? null : flags);

            var opts = new CreateServerOptions(
                Name: form.Name,
                MinecraftVersion: form.Version,
                ServerDirectory: form.Directory,
                Type: form.Type,
                Options: serverOpts,
                RconOptions: rconOpts,
                JvmOptions: jvmOpts,
                AcceptEula: form.AcceptEula);

            var server = await _service.CreateServerAsync(opts, progress, _appCts.Token);

            if (form.AcceptEula)
            {
                state.StatusMsg = "Accepting EULA...";
                state.Progress = 1.0;
                await _service.AcceptEulaAsync(server.Id, _appCts.Token);
            }

            await ServerListVm.RefreshAsync(_appCts.Token);
            Ui.Post(() => SetStatus($"Created: {form.Name}"));
            state.Progress = 1.0;
            state.StatusMsg = "Complete";
            state.IsComplete = true;
        }
        catch (Exception ex)
        {
            state.StatusMsg = $"Error: {ex.Message}";
            state.HasError = true;
            Ui.Post(() => SetStatus($"Create failed: {ex.Message}", true));
        }
    }
}
