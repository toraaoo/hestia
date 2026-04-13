using Hestia.Core;
using Hestia.Core.Abstractions;
using Hestia.Core.Server;
using Hestia.Tui.Formatting;
using Hestia.Tui.Input;
using Hestia.Tui.Services;
using Hestia.Tui.ViewModels;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.App;

internal sealed class TuiApp
{
    private enum Pane
    {
        Servers,
        JRE,
        Logs,
        Info,
        Command
    }

    private enum Tab
    {
        Logs,
        Info
    }

    private enum PendingAction
    {
        None,
        Create,
        DeleteServer
    }

    private enum CreateMode
    {
        Normal,
        EditText,
        SelectVersion,
        SelectType,
    }

    private const int HeaderH = 7;
    private const int JreH = 7;
    private const int LeftMinW = 44;
    private const int LeftMaxW = 64;
    private const int RightMinW = 60;

    private readonly IHestiaService _service;
    private readonly AppInfo _appInfo;
    private readonly string _stamp;
    private readonly UiDispatcher _ui = new();
    private readonly KeyMap _keyMap = KeyMap.Default();

    private ServerListVm _serverListVm = null!;
    private JreListVm _jreListVm = null!;
    private CommandVm _commandVm = null!;
    private ServerSessionVm? _session;

    private Guid? _selectedServerId;
    private int _serverCursor = 0;
    private Pane _activePane = Pane.Servers;
    private Tab _activeTab = Tab.Logs;

    private int _logScroll;

    private string _inputBuffer = string.Empty;
    private string _statusMsg = string.Empty;
    private bool _statusIsError;
    private bool _logFollow = true;
    private bool _showRconPassword;
    private bool _quit;
    private PendingAction _pendingAction = PendingAction.None;
    private Guid? _pendingDeleteId;

    private Layout? _rightLogsCmdLayout;

    private readonly CancellationTokenSource _appCts = new();

    public TuiApp(IHestiaService service, AppInfo appInfo, string stamp)
    {
        _service = service;
        _appInfo = appInfo;
        _stamp = stamp;
    }

    public async Task RunAsync()
    {
        _serverListVm = new ServerListVm(_service);
        _jreListVm = new JreListVm(_service);
        _commandVm = new CommandVm(_service);

        _commandVm.LineAppended += line => _ui.Post(() => AppendRconOutputToLogs(line));
        _commandVm.StatusChanged += msg => _ui.Post(() => SetStatus(msg, true));

        Console.CursorVisible = false;
        _ = Task.Run(LoadInitialAsync);

        while (!_quit)
        {
            _pendingAction = PendingAction.None;
            await RunLiveAsync();

            if (_pendingAction == PendingAction.Create)
                await RunCreateFlowAsync();
            else if (_pendingAction == PendingAction.DeleteServer && _pendingDeleteId.HasValue)
                await RunDeleteFlowAsync(_pendingDeleteId.Value);
        }

        Console.CursorVisible = true;
        _appCts.Cancel();
        if (_session is not null) await _session.DisposeAsync();
    }

    private async Task RunLiveAsync()
    {
        var layout = BuildLayout();
        try
        {
            await AnsiConsole.Live(layout)
                .AutoClear(false)
                .Overflow(VerticalOverflow.Ellipsis)
                .Cropping(VerticalOverflowCropping.Bottom)
                .StartAsync(async ctx =>
                {
                    while (!_quit && _pendingAction == PendingAction.None)
                    {
                        _ui.Drain();
                        HandleInput();
                        UpdateLayout(layout);
                        ctx.Refresh();
                        await Task.Delay(50);
                    }
                });
        }
        catch (OperationCanceledException) { }
    }

    private async Task RunCreateFlowAsync()
    {
        AnsiConsole.Clear();
        Console.CursorVisible = false;

        try
        {
            var initialType = ServerType.Vanilla;
            var versions = await _service.GetAvailableVersionsAsync(initialType, _appCts.Token);
            var form = new ServerCreateForm(_appInfo.AppDataDirectory, versions);
            form.SetType(initialType);

            var existing = await _service.GetServersAsync(_appCts.Token);
            var used = new HashSet<int>(existing.SelectMany(s => new[] { s.Options.Port, s.RconOptions.Port }));
            form.SetServerPort(FindNextFreePort(25565, used));
            used.Add(form.ServerPort);
            form.SetRconPort(FindNextFreePort(25575, used));
            var mode = CreateMode.Normal;

            var editBuffer = string.Empty;
            var editOriginal = string.Empty;

            var versionQuery = string.Empty;
            var versionCursor = 0;
            var versionOriginal = form.Version;

            var typeCursor = 0;
            var typeOriginal = form.Type;

            var createError = string.Empty;

            while (true)
            {
                var table = RenderCreateFormTable(
                    form,
                    mode,
                    editBuffer,
                    versionQuery,
                    versionCursor,
                    typeCursor,
                    versions);

                var help = mode switch
                {
                    CreateMode.Normal => "[dim]↑↓:nav  Tab:next  /:edit  Space:toggle  Enter:create  Esc:cancel[/]",
                    CreateMode.EditText => "[dim]Type to edit  Enter:confirm  Esc:cancel  Tab:confirm+next  Backspace:delete[/]",
                    CreateMode.SelectVersion => "[dim]↑↓:select  Type:search  Enter:confirm  Esc:cancel  Tab:confirm+next  Backspace:delete[/]",
                    CreateMode.SelectType => "[dim]↑↓:select  Enter:confirm  Esc:cancel  Tab:confirm+next[/]",
                    _ => "[dim][/]"
                };

                var helpMarkup = string.IsNullOrWhiteSpace(createError)
                    ? help
                    : $"[bold red]{Markup.Escape(createError)}[/]\n{help}";

                var layout = new Layout()
                    .SplitRows(new Layout("Form"), new Layout("Help").Size(string.IsNullOrWhiteSpace(createError) ? 2 : 3));
                layout["Form"].Update(new Align(table, HorizontalAlignment.Center, VerticalAlignment.Middle));
                layout["Help"].Update(new Align(new Markup(helpMarkup), HorizontalAlignment.Center));
                AnsiConsole.Clear();
                AnsiConsole.Write(layout);

                if (!Console.KeyAvailable) { await Task.Delay(50); continue; }
                var key = Console.ReadKey(true);
                var createAction = _keyMap.Translate(key);

                if (createAction == InputAction.Escape)
                {
                    if (mode == CreateMode.EditText)
                    {
                        editBuffer = editOriginal;
                        mode = CreateMode.Normal;
                        createError = string.Empty;
                        continue;
                    }

                    if (mode == CreateMode.SelectVersion)
                    {
                        form.SetVersion(versionOriginal);
                        versionQuery = string.Empty;
                        versionCursor = 0;
                        mode = CreateMode.Normal;
                        createError = string.Empty;
                        continue;
                    }

                    if (mode == CreateMode.SelectType)
                    {
                        form.SetType(typeOriginal);
                        typeCursor = 0;
                        mode = CreateMode.Normal;
                        createError = string.Empty;
                        continue;
                    }

                    return;
                }

                if (mode == CreateMode.Normal)
                {
                    if (createAction == InputAction.CursorUp) { form.MoveUp(); continue; }
                    if (createAction == InputAction.CursorDown) { form.MoveDown(); continue; }
                    if (createAction is InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
                    {
                        if (createAction == InputAction.CycleFocusPrev) form.MoveUp();
                        else form.MoveDown();
                        createError = string.Empty;
                        continue;
                    }

                    if (key.Key == ConsoleKey.Spacebar)
                    {
                        if (form.SelectedField == ServerCreateForm.Field.Eula)
                        {
                            form.ToggleEula();
                            createError = string.Empty;
                            continue;
                        }

                        if (form.SelectedField == ServerCreateForm.Field.OnlineMode)
                        {
                            form.ToggleOnlineMode();
                            createError = string.Empty;
                            continue;
                        }

                        if (form.SelectedField == ServerCreateForm.Field.Whitelist)
                        {
                            form.ToggleWhitelist();
                            createError = string.Empty;
                            continue;
                        }

                        if (form.SelectedField == ServerCreateForm.Field.RconEnabled)
                        {
                            form.ToggleRconEnabled();
                            createError = string.Empty;
                            continue;
                        }
                    }

                    if (createAction == InputAction.OpenCommand)
                    {
                        if (form.SelectedField == ServerCreateForm.Field.Version)
                        {
                            versionOriginal = form.Version;
                            versionQuery = string.Empty;
                            versionCursor = FindIndex(versions, form.Version);
                            if (versionCursor < 0) versionCursor = 0;
                            mode = CreateMode.SelectVersion;
                            createError = string.Empty;
                            continue;
                        }

                        if (form.SelectedField == ServerCreateForm.Field.Type)
                        {
                            var types = form.GetTypes();
                            typeOriginal = form.Type;
                            typeCursor = Array.IndexOf(types, form.Type);
                            if (typeCursor < 0) typeCursor = 0;
                            mode = CreateMode.SelectType;
                            createError = string.Empty;
                            continue;
                        }

                        if (form.SelectedField is ServerCreateForm.Field.Name
                            or ServerCreateForm.Field.Directory
                            or ServerCreateForm.Field.ServerPort
                            or ServerCreateForm.Field.MaxPlayers
                            or ServerCreateForm.Field.MotD
                            or ServerCreateForm.Field.ViewDistance
                            or ServerCreateForm.Field.LevelName
                            or ServerCreateForm.Field.Difficulty
                            or ServerCreateForm.Field.RconPort
                            or ServerCreateForm.Field.RconPassword
                            or ServerCreateForm.Field.RconTimeoutSeconds
                            or ServerCreateForm.Field.JvmMinMemory
                            or ServerCreateForm.Field.JvmMaxMemory
                            or ServerCreateForm.Field.JvmAdditionalFlags)
                        {
                            editOriginal = form.SelectedField switch
                            {
                                ServerCreateForm.Field.Name => form.Name,
                                ServerCreateForm.Field.Directory => form.Directory,
                                ServerCreateForm.Field.ServerPort => form.ServerPort.ToString(),
                                ServerCreateForm.Field.MaxPlayers => form.MaxPlayers.ToString(),
                                ServerCreateForm.Field.MotD => form.MotD,
                                ServerCreateForm.Field.ViewDistance => form.ViewDistance.ToString(),
                                ServerCreateForm.Field.LevelName => form.LevelName,
                                ServerCreateForm.Field.Difficulty => form.Difficulty,
                                ServerCreateForm.Field.RconPort => form.RconPort.ToString(),
                                ServerCreateForm.Field.RconPassword => form.RconPassword,
                                ServerCreateForm.Field.RconTimeoutSeconds => form.RconTimeoutSeconds.ToString(),
                                ServerCreateForm.Field.JvmMinMemory => form.JvmMinMemory,
                                ServerCreateForm.Field.JvmMaxMemory => form.JvmMaxMemory,
                                ServerCreateForm.Field.JvmAdditionalFlags => form.JvmAdditionalFlags,
                                _ => string.Empty
                            };
                            editBuffer = editOriginal;
                            mode = CreateMode.EditText;
                            createError = string.Empty;
                            continue;
                        }
                    }

                    if (createAction == InputAction.Confirm)
                    {
                        if (string.IsNullOrWhiteSpace(form.Name))
                        {
                            createError = "Server name required";
                            continue;
                        }

                        if (!IsValidPort(form.ServerPort))
                        {
                            createError = "Server port must be 1-65535";
                            continue;
                        }

                        if (form.MaxPlayers is < 1 or > 10_000)
                        {
                            createError = "Max players must be 1-10000";
                            continue;
                        }

                        if (form.ViewDistance is < 2 or > 32)
                        {
                            createError = "View distance must be 2-32";
                            continue;
                        }

                        if (string.IsNullOrWhiteSpace(form.LevelName))
                        {
                            createError = "Level name required";
                            continue;
                        }

                        if (string.IsNullOrWhiteSpace(form.Difficulty))
                        {
                            createError = "Difficulty required";
                            continue;
                        }

                        if (!IsValidPort(form.RconPort))
                        {
                            createError = "RCON port must be 1-65535";
                            continue;
                        }

                        if (form.ServerPort == form.RconPort)
                        {
                            createError = "Server port and RCON port must differ";
                            continue;
                        }

                        if (form.RconEnabled)
                        {
                            if (string.IsNullOrWhiteSpace(form.RconPassword))
                            {
                                createError = "RCON password required";
                                continue;
                            }

                            if (form.RconTimeoutSeconds is < 1 or > 120)
                            {
                                createError = "RCON timeout must be 1-120";
                                continue;
                            }
                        }

                        if (!form.AcceptEula)
                        {
                            createError = "You must accept the EULA to create";
                            continue;
                        }

                        createError = string.Empty;
                        break;
                    }

                    continue;
                }

                if (mode == CreateMode.EditText)
                {
                    if (createAction == InputAction.TextBackspace)
                    {
                        if (editBuffer.Length > 0) editBuffer = editBuffer[..^1];
                        continue;
                    }

                    if (createAction is InputAction.Confirm or InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
                    {
                        if (form.SelectedField == ServerCreateForm.Field.Name)
                        {
                            form.SetName(editBuffer);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.Directory)
                        {
                            form.SetDirectory(editBuffer);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.ServerPort)
                        {
                            if (!TryParsePort(editBuffer, out var p))
                            {
                                createError = "Server port must be 1-65535";
                                continue;
                            }

                            form.SetServerPort(p);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.MaxPlayers)
                        {
                            if (!int.TryParse(editBuffer.Trim(), out var mp) || mp is < 1 or > 10_000)
                            {
                                createError = "Max players must be 1-10000";
                                continue;
                            }

                            form.SetMaxPlayers(mp);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.MotD)
                        {
                            form.SetMotD(editBuffer);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.ViewDistance)
                        {
                            if (!int.TryParse(editBuffer.Trim(), out var vd) || vd is < 2 or > 32)
                            {
                                createError = "View distance must be 2-32";
                                continue;
                            }

                            form.SetViewDistance(vd);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.LevelName)
                        {
                            form.SetLevelName(editBuffer);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.Difficulty)
                        {
                            form.SetDifficulty(editBuffer);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.RconPort)
                        {
                            if (!TryParsePort(editBuffer, out var p))
                            {
                                createError = "RCON port must be 1-65535";
                                continue;
                            }

                            form.SetRconPort(p);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.RconPassword)
                        {
                            form.SetRconPassword(editBuffer);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.RconTimeoutSeconds)
                        {
                            if (!int.TryParse(editBuffer.Trim(), out var t) || t is < 1 or > 120)
                            {
                                createError = "RCON timeout must be 1-120";
                                continue;
                            }

                            form.SetRconTimeoutSeconds(t);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.JvmMinMemory)
                        {
                            form.SetJvmMinMemory(editBuffer);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.JvmMaxMemory)
                        {
                            form.SetJvmMaxMemory(editBuffer);
                        }
                        else if (form.SelectedField == ServerCreateForm.Field.JvmAdditionalFlags)
                        {
                            form.SetJvmAdditionalFlags(editBuffer);
                        }

                        editBuffer = string.Empty;
                        mode = CreateMode.Normal;
                        createError = string.Empty;

                        if (createAction == InputAction.CycleFocusPrev) form.MoveUp();
                        else if (createAction == InputAction.CycleFocusNext) form.MoveDown();

                        continue;
                    }

                    if (createAction == InputAction.TextInput)
                    {
                        editBuffer += key.KeyChar;
                        continue;
                    }

                    continue;
                }

                if (mode == CreateMode.SelectVersion)
                {
                    var filtered = FilterVersions(versions, versionQuery);
                    if (filtered.Count == 0)
                    {
                        versionCursor = 0;
                    }
                    else
                    {
                        versionCursor = Math.Clamp(versionCursor, 0, filtered.Count - 1);
                    }

                    if (createAction == InputAction.CursorUp)
                    {
                        if (versionCursor > 0) versionCursor--;
                        continue;
                    }

                    if (createAction == InputAction.CursorDown)
                    {
                        if (filtered.Count > 0 && versionCursor < filtered.Count - 1) versionCursor++;
                        continue;
                    }

                    if (createAction == InputAction.TextBackspace)
                    {
                        if (versionQuery.Length > 0) versionQuery = versionQuery[..^1];
                        versionCursor = 0;
                        continue;
                    }

                    if (createAction is InputAction.Confirm or InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
                    {
                        if (filtered.Count > 0)
                            form.SetVersion(filtered[versionCursor]);

                        versionQuery = string.Empty;
                        versionCursor = 0;
                        mode = CreateMode.Normal;
                        createError = string.Empty;

                        if (createAction == InputAction.CycleFocusPrev) form.MoveUp();
                        else if (createAction == InputAction.CycleFocusNext) form.MoveDown();

                        continue;
                    }

                    if (createAction == InputAction.OpenCommand)
                        continue;

                    if (createAction == InputAction.TextInput)
                    {
                        versionQuery += key.KeyChar;
                        versionCursor = 0;
                        continue;
                    }

                    continue;
                }

                if (mode == CreateMode.SelectType)
                {
                    var types = form.GetTypes();
                    typeCursor = Math.Clamp(typeCursor, 0, Math.Max(0, types.Length - 1));

                    if (createAction == InputAction.CursorUp)
                    {
                        if (typeCursor > 0) typeCursor--;
                        continue;
                    }

                    if (createAction == InputAction.CursorDown)
                    {
                        if (types.Length > 0 && typeCursor < types.Length - 1) typeCursor++;
                        continue;
                    }

                    if (createAction is InputAction.Confirm or InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
                    {
                        if (types.Length > 0)
                        {
                            var nextType = types[typeCursor];
                            form.SetType(nextType);
                            versions = await _service.GetAvailableVersionsAsync(nextType, _appCts.Token);
                            versionQuery = string.Empty;
                            versionCursor = FindIndex(versions, form.Version);
                            if (versionCursor < 0) versionCursor = 0;
                        }

                        mode = CreateMode.Normal;
                        createError = string.Empty;

                        if (createAction == InputAction.CycleFocusPrev) form.MoveUp();
                        else if (createAction == InputAction.CycleFocusNext) form.MoveDown();

                        continue;
                    }

                    continue;
                }
            }

            await CreateServerAsync(
                form.Name,
                form.Version,
                form.Directory,
                form.Type,
                form.ServerPort,
                form.MaxPlayers,
                form.MotD,
                form.ViewDistance,
                form.OnlineMode,
                form.Whitelist,
                form.LevelName,
                form.Difficulty,
                form.RconEnabled,
                form.RconPort,
                form.RconPassword,
                form.RconTimeoutSeconds,
                form.JvmMinMemory,
                form.JvmMaxMemory,
                form.JvmAdditionalFlags,
                form.AcceptEula);
        }
        finally
        {
            Console.CursorVisible = false;
            AnsiConsole.Clear();
        }
    }

    private static bool IsValidPort(int port) => port is >= 1 and <= 65535;

    private static bool TryParsePort(string value, out int port)
    {
        port = 0;
        return int.TryParse(value.Trim(), out port) && IsValidPort(port);
    }

    private static int FindNextFreePort(int start, HashSet<int> used)
    {
        for (var p = Math.Max(1, start); p <= 65535; p++)
            if (!used.Contains(p))
                return p;

        return start;
    }

    private Table RenderCreateFormTable(
        ServerCreateForm form,
        CreateMode mode,
        string editBuffer,
        string versionQuery,
        int versionCursor,
        int typeCursor,
        IReadOnlyList<string> allVersions)
    {
        var table = new Table()
            .HideHeaders()
            .NoBorder()
            .AddColumn(new TableColumn(string.Empty).Width(60));

        var muted = mode is CreateMode.SelectVersion or CreateMode.SelectType;

        string Value(ServerCreateForm.Field field)
        {
            if (mode == CreateMode.EditText && form.SelectedField == field)
                return Markup.Escape(editBuffer) + "[dim]█[/]";
            return field switch
            {
                ServerCreateForm.Field.Name => Markup.Escape(form.Name),
                ServerCreateForm.Field.Type => Markup.Escape(form.Type.ToString()),
                ServerCreateForm.Field.Version => Markup.Escape(form.Version),
                ServerCreateForm.Field.Directory => Markup.Escape(form.Directory),
                ServerCreateForm.Field.ServerPort => form.ServerPort.ToString(),
                ServerCreateForm.Field.MaxPlayers => form.MaxPlayers.ToString(),
                ServerCreateForm.Field.MotD => Markup.Escape(form.MotD),
                ServerCreateForm.Field.ViewDistance => form.ViewDistance.ToString(),
                ServerCreateForm.Field.OnlineMode => form.OnlineMode ? "[green]ON[/]" : "[red]OFF[/]",
                ServerCreateForm.Field.Whitelist => form.Whitelist ? "[green]ON[/]" : "[red]OFF[/]",
                ServerCreateForm.Field.LevelName => Markup.Escape(form.LevelName),
                ServerCreateForm.Field.Difficulty => Markup.Escape(form.Difficulty),
                ServerCreateForm.Field.RconEnabled => form.RconEnabled ? "[green]ON[/]" : "[red]OFF[/]",
                ServerCreateForm.Field.RconPort => form.RconPort.ToString(),
                ServerCreateForm.Field.RconPassword => Markup.Escape(form.RconPassword),
                ServerCreateForm.Field.RconTimeoutSeconds => form.RconTimeoutSeconds.ToString(),
                ServerCreateForm.Field.JvmMinMemory => Markup.Escape(form.JvmMinMemory),
                ServerCreateForm.Field.JvmMaxMemory => Markup.Escape(form.JvmMaxMemory),
                ServerCreateForm.Field.JvmAdditionalFlags => Markup.Escape(form.JvmAdditionalFlags),
                ServerCreateForm.Field.Eula => form.AcceptEula ? "[green]ON[/]" : "[red]OFF[/]",
                _ => string.Empty
            };
        }

        var fields = new[]
        {
            ("Name", Value(ServerCreateForm.Field.Name), ServerCreateForm.Field.Name),
            ("Type", Value(ServerCreateForm.Field.Type), ServerCreateForm.Field.Type),
            ("Version", Value(ServerCreateForm.Field.Version), ServerCreateForm.Field.Version),
            ("Directory", Value(ServerCreateForm.Field.Directory), ServerCreateForm.Field.Directory),
            ("Server Port", Value(ServerCreateForm.Field.ServerPort), ServerCreateForm.Field.ServerPort),
            ("Max Players", Value(ServerCreateForm.Field.MaxPlayers), ServerCreateForm.Field.MaxPlayers),
            ("MotD", Value(ServerCreateForm.Field.MotD), ServerCreateForm.Field.MotD),
            ("View Dist", Value(ServerCreateForm.Field.ViewDistance), ServerCreateForm.Field.ViewDistance),
            ("Online", Value(ServerCreateForm.Field.OnlineMode), ServerCreateForm.Field.OnlineMode),
            ("Whitelist", Value(ServerCreateForm.Field.Whitelist), ServerCreateForm.Field.Whitelist),
            ("Level", Value(ServerCreateForm.Field.LevelName), ServerCreateForm.Field.LevelName),
            ("Difficulty", Value(ServerCreateForm.Field.Difficulty), ServerCreateForm.Field.Difficulty),
            ("RCON", Value(ServerCreateForm.Field.RconEnabled), ServerCreateForm.Field.RconEnabled),
            ("RCON Port", Value(ServerCreateForm.Field.RconPort), ServerCreateForm.Field.RconPort),
            ("RCON Pass", Value(ServerCreateForm.Field.RconPassword), ServerCreateForm.Field.RconPassword),
            ("RCON T/O", Value(ServerCreateForm.Field.RconTimeoutSeconds), ServerCreateForm.Field.RconTimeoutSeconds),
            ("Xms", Value(ServerCreateForm.Field.JvmMinMemory), ServerCreateForm.Field.JvmMinMemory),
            ("Xmx", Value(ServerCreateForm.Field.JvmMaxMemory), ServerCreateForm.Field.JvmMaxMemory),
            ("JVM Flags", Value(ServerCreateForm.Field.JvmAdditionalFlags), ServerCreateForm.Field.JvmAdditionalFlags),
            ("Accept EULA", Value(ServerCreateForm.Field.Eula), ServerCreateForm.Field.Eula)
        };

        foreach (var (label, value, fieldEnum) in fields)
        {
            var isSelected = form.SelectedField == fieldEnum && mode is CreateMode.Normal or CreateMode.EditText;
            var prefix = isSelected ? "→ " : "  ";
            var labelStyle = isSelected ? "bold yellow" : (muted ? "dim" : "white");
            var valueStyle = isSelected ? "bold cyan" : (muted ? "dim" : "cyan");

            var line = $"[{labelStyle}]{prefix}{label,-12}[/] [{valueStyle}]{value,-40}[/]";
            table.AddRow(new Markup(line));
        }

        if (mode == CreateMode.SelectVersion)
        {
            table.AddRow(new Markup("[dim]────────────────────────────────────────────────────────────[/]"));

            var filtered = FilterVersions(allVersions, versionQuery);
            const int pageSize = 8;
            var cur = filtered.Count == 0 ? 0 : Math.Clamp(versionCursor, 0, filtered.Count - 1);
            var start = Math.Max(0, cur - (pageSize / 2));
            var end = Math.Min(filtered.Count, start + pageSize);
            if (end - start < pageSize && start > 0) start = Math.Max(0, end - pageSize);

            if (start > 0)
                table.AddRow(new Markup("[dim]  ...[/]"));

            for (var i = start; i < end; i++)
            {
                var v = Markup.Escape(filtered[i]);
                var sel = i == cur;
                var prefix = sel ? "→ " : "  ";
                var style = sel ? "bold cyan" : "white";
                table.AddRow(new Markup($"[{style}]{prefix}{v}[/]"));
            }

            if (end < filtered.Count)
                table.AddRow(new Markup("[dim]  ...[/]"));

            var q = Markup.Escape(versionQuery);
            table.AddRow(new Markup($"[dim]Search:[/] [bold]{q}[/][dim]█[/]"));
        }
        else if (mode == CreateMode.SelectType)
        {
            table.AddRow(new Markup("[dim]────────────────────────────────────────────────────────────[/]"));
            var types = form.GetTypes();
            var cur = Math.Clamp(typeCursor, 0, Math.Max(0, types.Length - 1));
            for (var i = 0; i < types.Length; i++)
            {
                var t = Markup.Escape(types[i].ToString());
                var sel = i == cur;
                var prefix = sel ? "→ " : "  ";
                var style = sel ? "bold cyan" : "white";
                table.AddRow(new Markup($"[{style}]{prefix}{t}[/]"));
            }
        }

        return table;
    }

    private static int FindIndex(IReadOnlyList<string> list, string value)
    {
        for (var i = 0; i < list.Count; i++)
            if (list[i] == value) return i;
        return -1;
    }

    private static List<string> FilterVersions(IReadOnlyList<string> all, string query)
    {
        if (string.IsNullOrWhiteSpace(query))
            return [.. all];

        var q = query.Trim();
        var res = new List<string>(all.Count);
        for (var i = 0; i < all.Count; i++)
        {
            var v = all[i];
            if (v.Contains(q, StringComparison.OrdinalIgnoreCase))
                res.Add(v);
        }

        return res;
    }

    private Layout BuildLayout()
    {
        var root = new Layout("Root")
            .SplitRows(
                new Layout("Content"),
                new Layout("Status").Size(1));

        root["Content"].SplitColumns(
            new Layout("Left").Size(ComputeLeftWidth(Console.WindowWidth)),
            new Layout("Right"));

        root["Left"].SplitRows(
            new Layout("Header").Size(HeaderH),
            new Layout("Servers"),
            new Layout("JRE").Size(JreH));

        root["Header"].Update(RenderHeader());
        root["Servers"].Update(RenderServerList());
        root["JRE"].Update(RenderJreList());
        UpdateRight(root["Right"]);
        root["Status"].Update(RenderStatus());

        return root;
    }

    private void UpdateLayout(Layout layout)
    {
        layout["Left"].Size(ComputeLeftWidth(Console.WindowWidth));
        layout["Header"].Update(RenderHeader());
        layout["Servers"].Update(RenderServerList());
        layout["JRE"].Update(RenderJreList());
        UpdateRight(layout["Right"]);
        layout["Status"].Update(RenderStatus());
    }

    private void UpdateRight(Layout right)
    {
        if (_selectedServerId is null)
        {
            right.Update(new Panel(new Markup("[dim]Press [bold]Enter[/] to select a server.[/]"))
            {
                Header = new PanelHeader("No selection"),
                Border = BoxBorder.Rounded,
                Expand = true,
            });
            return;
        }

        if (_activeTab == Tab.Info)
        {
            right.Update(RenderInfoPanel());
            return;
        }

        _rightLogsCmdLayout ??= new Layout("RightLogsCmd")
            .SplitRows(
                new Layout("Logs"),
                new Layout("Command").Size(3));

        _rightLogsCmdLayout["Logs"].Update(RenderLogsPanel());
        _rightLogsCmdLayout["Command"].Update(RenderCommandPanel());
        right.Update(_rightLogsCmdLayout);
    }

    private IRenderable RenderHeader()
    {
        var content = new Markup(
            $"[bold green]{AsciiArt.Header}[/]\n[dim]{Markup.Escape(AsciiArt.Stamp(_appInfo.Version, _stamp))}[/]");
        return new Panel(content) { Border = BoxBorder.None, Expand = true };
    }

    private IRenderable RenderServerList()
    {
        var servers = _serverListVm.Servers;
        var focused = _activePane == Pane.Servers;

        IRenderable body;
        if (servers.Count == 0)
        {
            body = new Markup("[dim]No servers. Press [bold]c[/] to create one.[/]");
        }
        else
        {
            var table = new Table()
                .HideHeaders()
                .NoBorder()
                .AddColumn(new TableColumn(string.Empty).NoWrap())
                .Expand();

            for (var i = 0; i < servers.Count; i++)
            {
                var row = Markup.Escape(RowFormatters.ServerRow(servers[i]));
                table.AddRow(i == _serverCursor && focused
                    ? new Markup($"[bold reverse] {row} [/]")
                    : new Markup($" {row}"));
            }

            body = table;
        }

        return new Panel(body)
        {
            Header = new PanelHeader(focused ? "[bold]Servers[/]" : "Servers"),
            Border = focused ? BoxBorder.Double : BoxBorder.Rounded,
            Expand = true,
        };
    }

    private IRenderable RenderJreList()
    {
        var rows = _jreListVm.Rows;
        var focused = _activePane == Pane.JRE;

        IRenderable body;
        if (rows.Count == 0)
        {
            body = new Markup("[dim]No runtimes found.[/]");
        }
        else
        {
            var table = new Table()
                .HideHeaders()
                .NoBorder()
                .AddColumn(new TableColumn(string.Empty).NoWrap())
                .Expand();
            foreach (var r in rows)
                table.AddRow(new Markup(Markup.Escape(r)));
            body = table;
        }

        return new Panel(body)
        {
            Header = new PanelHeader(focused ? "[bold]Java Runtimes[/]" : "Java Runtimes"),
            Border = focused ? BoxBorder.Double : BoxBorder.Rounded,
            Expand = true,
        };
    }

    private IRenderable RenderLogsPanel()
    {
        var lines = _session?.LogBuffer.Snapshot() ?? [];
        var focused = _activePane == Pane.Logs;

        var logTable = new Table()
            .HideHeaders()
            .NoBorder()
            .AddColumn(new TableColumn(string.Empty).NoWrap())
            .Expand();

        if (lines.Count == 0)
        {
            logTable.AddRow(new Markup("[dim]No logs yet...[/]"));
        }
        else
        {
            var viewport = Math.Clamp(Console.WindowHeight - 14, 10, 200);
            if (_logFollow) _logScroll = 0;
            _logScroll = Math.Clamp(_logScroll, 0, Math.Max(0, lines.Count - 1));

            var end = Math.Clamp(lines.Count - _logScroll, 0, lines.Count);
            var start = Math.Max(0, end - viewport);

            if (_logScroll > 0)
                logTable.AddRow(new Markup("[dim]... (PgDn to newest)[/]"));

            for (var i = start; i < end; i++)
                logTable.AddRow(new Markup(Markup.Escape(lines[i])));
        }

        var tabBar = _activeTab == Tab.Logs
            ? "[bold underline]Logs[/]  [dim]Info[/]"
            : "[dim]Logs[/]  [bold underline]Info[/]";

        var headerSuffix = "[dim]←→:tab  f:follow  PgUp/PgDn:scroll[/]";

        return new Panel(logTable)
        {
            Header = new PanelHeader($"{tabBar}  {headerSuffix}"),
            Border = focused ? BoxBorder.Double : BoxBorder.Rounded,
            Expand = true,
        };
    }

    private IRenderable RenderCommandPanel()
    {
        var focused = _activePane == Pane.Command;
        var content = focused
            ? new Markup($"[bold]>[/] {Markup.Escape(_inputBuffer)}[blink]█[/]")
            : new Markup("[dim]/ → command[/]");

        return new Panel(content)
        {
            Header = new PanelHeader(focused ? "[bold]Command[/]" : "Command"),
            Border = focused ? BoxBorder.Double : BoxBorder.Rounded,
            Expand = true,
            Padding = new Padding(1, 0, 1, 0)
        };
    }

    private IRenderable RenderInfoPanel()
    {
        var s = _session?.LatestStatus;
        var focused = _activePane == Pane.Info;
        var server = _selectedServerId is { } id ? _serverListVm.Servers.FirstOrDefault(x => x.Id == id) : null;

        var grid = new Grid()
            .AddColumn(new GridColumn().NoWrap().Width(12))
            .AddColumn(new GridColumn().NoWrap());

        void Row(string label, string val)
            => grid.AddRow(new Markup($"[dim]{label}[/]"), new Markup(val));

        if (server is not null)
        {
            var host = "127.0.0.1";
            Row("Name", Markup.Escape(server.Name));
            Row("Type", Markup.Escape(server.Type.ToString()));
            Row("Version", Markup.Escape(server.MinecraftVersion));
            Row("Dir", $"[dim]{Markup.Escape(server.Options.ServerDirectory)}[/]");
            Row("Port", server.Options.Port.ToString());
            Row("Join", $"{host}:{server.Options.Port}");

            if (server.RconOptions.Enabled)
            {
                var pw = _showRconPassword
                    ? server.RconOptions.Password
                    : new string('*', Math.Clamp(server.RconOptions.Password.Length, 8, 24));
                Row("RCON", "[green]ON[/]");
                Row("RCON cmd", Markup.Escape($"mcrcon -H {host} -P {server.RconOptions.Port} -p {pw}"));
            }
            else
            {
                Row("RCON", "[red]OFF[/]");
            }
            grid.AddEmptyRow();
        }

        if (s is null)
        {
            Row("State", "[dim]---[/]");
            Row("Uptime", "[dim]---[/]");
            Row("Players", "[dim]---[/]");
            Row("TPS", "[dim]---[/]");
            Row("Memory", "[dim]---[/]");
            Row("CPU", "[dim]---[/]");
        }
        else
        {
            var stateColor = s.State switch
            {
                ServerState.Running => "green",
                ServerState.Crashed => "red",
                ServerState.Starting => "yellow",
                _ => "dim",
            };
            var uptime = s.Uptime is { } u
                ? $"{(int)u.TotalHours:D2}:{u.Minutes:D2}:{u.Seconds:D2}"
                : "--:--:--";

            Row("State", $"[{stateColor}]{s.State}[/]");
            Row("Uptime", uptime);
            Row("Players", $"{s.PlayerCount}/{s.MaxPlayers}");
            Row("TPS", s.Tps is { } t ? $"{t:F1}" : "[dim]N/A[/]");
            Row("Memory", s.Resources is { } r
                ? $"{r.MemoryBytes / 1024 / 1024} MB / {r.MemoryLimitBytes / 1024 / 1024} MB"
                : "[dim]N/A[/]");
            Row("CPU", s.Resources is { } rc
                ? $"{rc.CpuPercent:F1}%"
                : "[dim]N/A[/]");

            if (s.OnlinePlayers.Count > 0)
            {
                grid.AddEmptyRow();
                grid.AddRow(
                    new Markup("[dim]Online[/]"),
                    new Markup(Markup.Escape(string.Join(", ", s.OnlinePlayers.Select(p => p.Username)))));
            }
        }

        var tabBar = "[dim]Logs[/]  [bold underline]Info[/]";
        return new Panel(grid)
        {
            Header = new PanelHeader(tabBar),
            Border = focused ? BoxBorder.Double : BoxBorder.Rounded,
            Expand = true,
        };
    }

    private IRenderable RenderStatus()
    {
        var hints = _activePane == Pane.Command
            ? "[dim]Esc:cancel  Enter:send  ↑↓:history[/]"
            : "[dim]q:quit  s:start  t:stop  r:restart  d:delete  c:create  p:pw  Tab:focus  /:command  ←→:tab  Esc:back[/]";

        if (!string.IsNullOrEmpty(_statusMsg))
        {
            var style = _statusIsError ? "bold red" : "bold green";
            return new Markup($"{hints}  [{style}]{Markup.Escape(_statusMsg)}[/]");
        }

        return new Markup(hints);
    }

    private void HandleInput()
    {
        while (Console.KeyAvailable)
        {
            var key = Console.ReadKey(intercept: true);
            ProcessKey(key);
        }
    }

    private void ProcessKey(ConsoleKeyInfo k)
    {
        var action = _keyMap.Translate(k);

        if (_activePane == Pane.Command)
        {
            HandleCommandKey(k, action);
            return;
        }

        switch (action)
        {
            case InputAction.OpenCommand when _selectedServerId.HasValue:
                _activePane = Pane.Command;
                return;
            case InputAction.Quit:
                _quit = true;
                return;
            case InputAction.TogglePassword when _selectedServerId.HasValue:
                _showRconPassword = !_showRconPassword;
                return;
            case InputAction.Escape when _selectedServerId.HasValue:
                _ = DeselectAsync();
                return;
            case InputAction.CycleFocusNext:
                CycleFocus(1);
                return;
            case InputAction.CycleFocusPrev:
                CycleFocus(-1);
                return;
            case InputAction.TabLeft when _activePane is Pane.Logs or Pane.Info:
                _activeTab = Tab.Logs;
                _activePane = Pane.Logs;
                return;
            case InputAction.TabRight when _activePane is Pane.Logs or Pane.Info:
                _activeTab = Tab.Info;
                _activePane = Pane.Info;
                return;
        }

        if (_activePane == Pane.Servers)
            HandleServersKey(action);

        if (_activePane == Pane.Logs)
        {
            if (action == InputAction.PageUp) { ScrollLogs(+5); return; }
            if (action == InputAction.PageDown) { ScrollLogs(-5); return; }
        }
    }

    private void HandleServersKey(InputAction? action)
    {
        switch (action)
        {
            case InputAction.CursorUp:
                _serverCursor = Math.Max(0, _serverCursor - 1);
                break;
            case InputAction.CursorDown:
                _serverCursor = Math.Min(_serverListVm.Servers.Count - 1, _serverCursor + 1);
                break;
            case InputAction.Confirm:
            {
                var s = _serverListVm.GetAt(_serverCursor);
                if (s is not null) _ = SelectServerAsync(s.Id);
                break;
            }
            case InputAction.ServerStart:   RunServerAction('s'); break;
            case InputAction.ServerStop:    RunServerAction('t'); break;
            case InputAction.ServerRestart: RunServerAction('r'); break;
            case InputAction.ServerDelete:  DeleteServer(); break;
            case InputAction.ServerCreate:
                _pendingAction = PendingAction.Create;
                break;
            case InputAction.ToggleFollow:
                _logFollow = !_logFollow;
                break;
        }
    }

    private void HandleCommandKey(ConsoleKeyInfo k, InputAction? action)
    {
        switch (action)
        {
            case InputAction.Confirm:
            {
                var cmd = _inputBuffer;
                _inputBuffer = string.Empty;
                if (!string.IsNullOrWhiteSpace(cmd) && _selectedServerId is { } id)
                {
                    _logFollow = true;
                    _logScroll = 0;
                    _ = _commandVm.SendAsync(id, cmd, _appCts.Token);
                }
                break;
            }
            case InputAction.Escape:
                _inputBuffer = string.Empty;
                _activePane = Pane.Logs;
                break;
            case InputAction.PageUp:
                ScrollLogs(+5);
                break;
            case InputAction.PageDown:
                ScrollLogs(-5);
                break;
            case InputAction.TextBackspace:
                if (_inputBuffer.Length > 0)
                    _inputBuffer = _inputBuffer[..^1];
                break;
            case InputAction.CursorUp:
            {
                var cur = _inputBuffer;
                _commandVm.HistoryUp(ref cur);
                _inputBuffer = cur;
                break;
            }
            case InputAction.CursorDown:
            {
                var cur = _inputBuffer;
                _commandVm.HistoryDown(ref cur);
                _inputBuffer = cur;
                break;
            }
            case InputAction.TextInput:
                _inputBuffer += k.KeyChar;
                break;
        }
    }

    private void ScrollLogs(int delta)
    {
        var lines = _session?.LogBuffer.Snapshot() ?? [];
        var viewport = Math.Clamp(Console.WindowHeight - 14, 10, 200);
        var maxScroll = Math.Max(0, lines.Count - viewport);

        if (delta > 0) _logFollow = false;
        _logScroll = Math.Clamp(_logScroll + delta, 0, maxScroll);
        if (_logScroll == 0) _logFollow = true;
    }

    private void AppendRconOutputToLogs(string text)
    {
        if (_session is null) return;

        var s = text.Replace("\r", string.Empty);
        var parts = s.Split('\n');
        for (var i = 0; i < parts.Length; i++)
        {
            var p = parts[i];
            if (p.Length == 0) continue;
            _session.LogBuffer.Add(p);
        }
    }

    private void CycleFocus(int dir)
    {
        Pane[] panes = _selectedServerId.HasValue
            ? [Pane.Servers, Pane.JRE, Pane.Logs, Pane.Info]
            : [Pane.Servers, Pane.JRE];

        var idx = Array.IndexOf(panes, _activePane);
        idx = ((idx + dir) % panes.Length + panes.Length) % panes.Length;
        _activePane = panes[idx];

        if (_activePane == Pane.Logs) _activeTab = Tab.Logs;
        if (_activePane == Pane.Info) _activeTab = Tab.Info;
    }


    private async Task LoadInitialAsync()
    {
        try
        {
            await Task.WhenAll(
                _serverListVm.RefreshAsync(_appCts.Token),
                _jreListVm.RefreshAsync(_appCts.Token));
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            _ui.Post(() => SetStatus($"Load error: {ex.Message}", true));
        }

        _ = PollServerListAsync();
    }

    private async Task PollServerListAsync()
    {
        while (!_appCts.IsCancellationRequested)
        {
            try
            {
                await Task.Delay(2000, _appCts.Token);
                await _serverListVm.RefreshAsync(_appCts.Token);
                _ui.Post(() =>
                {
                    if (_serverListVm.Servers.Count > 0)
                        _serverCursor = Math.Min(_serverCursor, _serverListVm.Servers.Count - 1);
                });
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch { }
        }
    }

    private async Task SelectServerAsync(Guid id)
    {
        if (_session is not null)
        {
            await _session.DisposeAsync();
            _session = null;
        }

        _selectedServerId = id;
        _activePane = Pane.Logs;
        _activeTab = Tab.Logs;

        var name = _serverListVm.Servers.FirstOrDefault(s => s.Id == id)?.Name ?? id.ToString()[..8];
        SetStatus($"Selected: {name}");

        _session = new ServerSessionVm(_service, _ui, id);
        _session.Start();
    }

    private async Task DeselectAsync()
    {
        if (_session is not null)
        {
            await _session.DisposeAsync();
            _session = null;
        }

        _selectedServerId = null;
        _activePane = Pane.Servers;
        SetStatus(string.Empty);
    }

    private void DeleteServer()
    {
        var server = _serverListVm.GetAt(_serverCursor);
        if (server is null) return;

        _pendingDeleteId = server.Id;
        _pendingAction = PendingAction.DeleteServer;
    }

    private async Task RunDeleteFlowAsync(Guid serverId)
    {
        var server = _serverListVm.Servers.FirstOrDefault(s => s.Id == serverId);
        if (server is null) return;

        Console.CursorVisible = true;

        try
        {
            var table = new Table()
                .HideHeaders()
                .NoBorder()
                .AddColumn(new TableColumn(string.Empty).Centered());

            table.AddRow(new Markup("[bold red]Delete server?[/]"));
            table.AddRow(new Markup($"[dim]{server.Name}[/]"));

            var layout = new Layout()
                .SplitRows(
                    new Layout("Form"),
                    new Layout("Help").Size(2));

            layout["Form"].Update(new Align(table, HorizontalAlignment.Center, VerticalAlignment.Middle));
            layout["Help"].Update(new Align(new Markup("[dim]Y:confirm  N:cancel[/]"), HorizontalAlignment.Center));

            AnsiConsole.Clear();
            AnsiConsole.Write(layout);

            while (true)
            {
                if (!Console.KeyAvailable)
                {
                    await Task.Delay(50);
                    continue;
                }

                var key = Console.ReadKey(true);
                if (key.KeyChar == 'y' || key.KeyChar == 'Y')
                    break;
                if (key.KeyChar == 'n' || key.KeyChar == 'N')
                    return;
            }

            Console.CursorVisible = false;
            SetStatus($"Deleting {server.Name}…");

            await _serverListVm.DeleteAsync(serverId, _appCts.Token);
            _ui.Post(() =>
            {
                SetStatus($"Deleted: {server.Name}");
                _selectedServerId = null;
                _activePane = Pane.Servers;
            });
        }
        catch (Exception ex)
        {
            _ui.Post(() => SetStatus($"Error: {ex.Message}", true));
        }
        finally
        {
            Console.CursorVisible = false;
            AnsiConsole.Clear();
        }
    }

    private void RunServerAction(char key)
    {
        var server = _serverListVm.GetAt(_serverCursor);
        if (server is null) return;

        var task = key switch
        {
            's' => _serverListVm.StartAsync(server.Id, _appCts.Token),
            't' => _serverListVm.StopAsync(server.Id, _appCts.Token),
            'r' => _serverListVm.RestartAsync(server.Id, _appCts.Token),
            _ => Task.CompletedTask,
        };
        var verb = key switch { 's' => "Starting", 't' => "Stopping", 'r' => "Restarting", _ => "" };

        SetStatus($"{verb} {server.Name}…");

        _ = task.ContinueWith(t =>
        {
            if (t.IsFaulted)
            {
                _ui.Post(() => SetStatus($"Error: {t.Exception?.InnerException?.Message}", true));
                return;
            }

            if (key is 's' or 'r')
                _ = SelectServerAsync(server.Id);
            else
                _ui.Post(() => SetStatus($"{server.Name}: done"));
        }, TaskScheduler.Default);
    }

    private async Task CreateServerAsync(
        string name,
        string version,
        string dir,
        ServerType type,
        int serverPort,
        int maxPlayers,
        string motd,
        int viewDistance,
        bool onlineMode,
        bool whitelist,
        string levelName,
        string difficulty,
        bool rconEnabled,
        int rconPort,
        string rconPassword,
        int rconTimeoutSeconds,
        string jvmMinMemory,
        string jvmMaxMemory,
        string jvmAdditionalFlags,
        bool acceptEula)
    {
        SetStatus($"Creating {name}…");
        try
        {
            var serverOpts = new ServerOptions(
                ServerDirectory: dir,
                Port: serverPort,
                MaxPlayers: maxPlayers,
                MotD: motd,
                ViewDistance: viewDistance,
                OnlineMode: onlineMode,
                Whitelist: whitelist,
                LevelName: levelName,
                Difficulty: difficulty);

            var rconOpts = new RconOptions(
                Port: rconPort,
                Password: rconPassword,
                Enabled: rconEnabled,
                ConnectTimeoutSeconds: rconTimeoutSeconds);

            var flags = SplitJvmFlags(jvmAdditionalFlags);
            var jvmOpts = new JvmOptions(
                MinMemory: jvmMinMemory,
                MaxMemory: jvmMaxMemory,
                AdditionalFlags: flags.Count == 0 ? null : flags);

            var opts = new CreateServerOptions(
                Name: name, MinecraftVersion: version, ServerDirectory: dir,
                Type: type,
                Options: serverOpts,
                RconOptions: rconOpts,
                JvmOptions: jvmOpts,
                AcceptEula: acceptEula);

            var server = await _service.CreateServerAsync(opts, ct: _appCts.Token);
            if (acceptEula) await _service.AcceptEulaAsync(server.Id, _appCts.Token);
            await _serverListVm.RefreshAsync(_appCts.Token);
            _ui.Post(() => SetStatus($"Created: {name}"));
        }
        catch (Exception ex)
        {
            _ui.Post(() => SetStatus($"Create failed: {ex.Message}", true));
        }
    }

    private static List<string> SplitJvmFlags(string raw)
    {
        var s = raw?.Trim();
        if (string.IsNullOrWhiteSpace(s)) return [];
        return s.Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries).ToList();
    }

    private static int ComputeLeftWidth(int totalW)
    {
        var target = (int)Math.Round(totalW * 0.45);
        target = Math.Clamp(target, LeftMinW, LeftMaxW);

        var maxLeft = totalW - RightMinW;
        if (maxLeft >= LeftMinW)
            return Math.Min(target, maxLeft);

        return Math.Max(20, totalW / 2);
    }

    private void SetStatus(string msg, bool isError = false)
    {
        _statusMsg = msg;
        _statusIsError = isError;
    }
}
