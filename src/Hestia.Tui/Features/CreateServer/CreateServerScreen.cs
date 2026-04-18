using System.Text.RegularExpressions;
using Hestia.Core;
using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Core.Utils;
using Hestia.Tui.Input;
using Hestia.Tui.Modals;
using Hestia.Tui.Navigation;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.CreateServer;

public sealed class CreateServerScreen(Manager manager, INavigator navigator) : ScreenBase
{
    private enum Step
    {
        Form,
        Creating,
        Done,
        Error
    }

    // Tab 0 – Essential : Name(0) Type(1) Version(2) [Create](3)
    // Tab 1 – Network   : Port(0) MaxPlayers(1) MotD(2) ViewDist(3) OnlineMode(4) Whitelist(5) [Create](6)
    // Tab 2 – RCON      : Enabled(0) Port(1) Password(2) Timeout(3) [Create](4)
    // Tab 3 – JVM       : Min(0) Max(1) [Create](2)
    // Tab 4 – World     : WorldName(0) Seed(1) GameMode(2) Difficulty(3) [Create](4)
    private static readonly string[] TabNames = ["Essential", "Network", "RCON", "JVM", "World"];
    private static readonly int[] TabFieldCounts = [4, 7, 5, 3, 5];

    private int _tabIndex;
    private readonly int[] _fieldCursors = new int[5];
    private readonly int[] _offsets = new int[5];

    // ── Essential ─────────────────────────────────────────────────────
    private string _name = "";
    private ServerType _type = ServerType.Vanilla;
    private MinecraftVersion? _selectedVersion;

    // ── Network ───────────────────────────────────────────────────────
    private string _port = "25565";
    private string _maxPlayers = "20";
    private string _motd = "A Minecraft Server";
    private string _viewDistance = "10";
    private bool _onlineMode = true;
    private bool _whitelist = false;

    // ── RCON ──────────────────────────────────────────────────────────
    private bool _rconEnabled = false;
    private string _rconPort = "25575";
    private string _rconPassword = "";
    private string _rconTimeout = "10";

    // ── JVM ───────────────────────────────────────────────────────────
    private string _jvmMin = "512M";
    private string _jvmMax = "2G";

    // ── World ─────────────────────────────────────────────────────────
    private string _worldName = "world";
    private string _worldSeed = "";
    private GameMode _gameMode = GameMode.Survival;
    private Difficulty _difficulty = Difficulty.Normal;

    // ── Step state ────────────────────────────────────────────────────
    private Step _step = Step.Form;
    private double _progress;
    private string _progressLabel = "";
    private string? _errorMessage;
    private Layout? _layout;

    // ─────────────────────────────────────────────────────────────────
    // Load
    // ─────────────────────────────────────────────────────────────────

    public override Task LoadAsync(CancellationToken ct)
    {
        var existing = manager.List().ToList();
        var occupied = existing.Select(s => s.Network.Port)
            .Concat(existing.Where(s => s.Rcon.Enabled).Select(s => s.Rcon.Port))
            .ToHashSet();

        _port = FindFreePort(occupied, 25565).ToString();
        _rconPort = FindFreePort(occupied, 25575).ToString();
        return Task.CompletedTask;
    }

    private static int FindFreePort(HashSet<int> occupied, int start)
    {
        while (occupied.Contains(start)) start++;
        return start;
    }

    // ─────────────────────────────────────────────────────────────────
    // Raw key input
    // ─────────────────────────────────────────────────────────────────

    public override bool OnRawKey(ConsoleKeyInfo key)
    {
        if (_step != Step.Form) return false;

        var text = ActiveTextField();
        if (text is null) return false;

        if (key.Key == ConsoleKey.Backspace)
        {
            if (text.Value.Length == 0) return false;
            text.Value = text.Value[..^1];
            return true;
        }

        if (char.IsControl(key.KeyChar)) return false;
        text.Value += key.KeyChar;
        return true;
    }

    // ─────────────────────────────────────────────────────────────────
    // Action input
    // ─────────────────────────────────────────────────────────────────

    public override void OnInput(InputAction action)
    {
        switch (_step)
        {
            case Step.Form:
                HandleFormInput(action);
                break;
            case Step.Error:
                if (action is InputAction.Back or InputAction.Confirm or InputAction.Quit)
                    _step = Step.Form;
                break;
        }
    }

    private void HandleFormInput(InputAction action)
    {
        switch (action)
        {
            case InputAction.Quit:
                navigator.Pop();
                return;
            case InputAction.Back:
                if (_tabIndex > 0) _tabIndex--;
                else navigator.Pop();
                return;
            case InputAction.Tab:
                _tabIndex = (_tabIndex + 1) % TabNames.Length;
                return;
            case InputAction.MoveUp:
                MoveCursor(-1);
                return;
            case InputAction.MoveDown:
                MoveCursor(1);
                return;
            case InputAction.Confirm:
                HandleConfirm();
                return;
            case InputAction.MoveLeft or InputAction.MoveRight:
                HandleBoolToggle();
                return;
        }
    }

    private void MoveCursor(int dir)
    {
        var max = TabFieldCounts[_tabIndex] - 1;
        _fieldCursors[_tabIndex] = Math.Clamp(_fieldCursors[_tabIndex] + dir, 0, max);

        var windowSize = Math.Max(3, Console.WindowHeight - 8);
        var cursor = _fieldCursors[_tabIndex];
        if (cursor < _offsets[_tabIndex])
            _offsets[_tabIndex] = cursor;
        else if (cursor >= _offsets[_tabIndex] + windowSize)
            _offsets[_tabIndex] = cursor - windowSize + 1;
    }

    private void HandleConfirm()
    {
        var cursor = _fieldCursors[_tabIndex];
        var isCreate = cursor == TabFieldCounts[_tabIndex] - 1;
        if (isCreate)
        {
            navigator.ShowModal(
                new ConfirmModal(
                    "By creating this server you agree to the [bold]Minecraft EULA[/].\n" +
                    "https://www.minecraft.net/en-us/eula\n\n" +
                    "Hestia will automatically accept it on your behalf."
                ),
                accepted =>
                {
                    if (accepted) RunCreate();
                });
            return;
        }

        switch (_tabIndex, cursor)
        {
            case (0, 1):
                navigator.ShowModal(
                    new ListSelectModal<ServerType>("Server Type", Enum.GetValues<ServerType>(), t => t.ToString()!),
                    result =>
                    {
                        if (!result.HasValue) return;

                        _type = result.Value;
                        _selectedVersion = null;
                    });
                break;
            case (0, 2):
            {
                var serverType = _type;
                navigator.ShowModal(
                    new ListSelectModal<MinecraftVersion>(
                        $"Pick Version ({serverType})",
                        _ => manager.GetAvailableVersionsAsync(serverType),
                        v => v.Version,
                        filters: [("Hide Snapshots", (MinecraftVersion v) => !v.IsSnapshot)]
                    ),
                    result =>
                    {
                        if (result.HasValue) _selectedVersion = result.Value;
                    });
                break;
            }
            case (1, 4): _onlineMode = !_onlineMode; break;
            case (1, 5): _whitelist = !_whitelist; break;
            case (2, 0): _rconEnabled = !_rconEnabled; break;
            case (4, 2):
                navigator.ShowModal(
                    new ListSelectModal<GameMode>("Game Mode", Enum.GetValues<GameMode>(), t => t.ToString()!),
                    result =>
                    {
                        if (result.HasValue) _gameMode = result.Value;
                    });
                break;
            case (4, 3):
                navigator.ShowModal(
                    new ListSelectModal<Difficulty>("Difficulty", Enum.GetValues<Difficulty>(), t => t.ToString()!),
                    result =>
                    {
                        if (result.HasValue) _difficulty = result.Value;
                    });
                break;
        }
    }

    private void HandleBoolToggle()
    {
        switch (_tabIndex, _fieldCursors[_tabIndex])
        {
            case (1, 4): _onlineMode = !_onlineMode; break;
            case (1, 5): _whitelist = !_whitelist; break;
            case (2, 0): _rconEnabled = !_rconEnabled; break;
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // Validation
    // ─────────────────────────────────────────────────────────────────

    private IEnumerable<string> Validate()
    {
        if (string.IsNullOrWhiteSpace(_name))
            yield return "Name is required";

        if (_selectedVersion is null)
            yield return "Version is required — open Essential tab and pick a version";

        if (!TryParsePort(_port))
            yield return $"Port must be 1–65535 (got '{_port}')";

        if (!int.TryParse(_maxPlayers, out var mp) || mp < 1)
            yield return $"Max Players must be a positive integer (got '{_maxPlayers}')";

        if (!int.TryParse(_viewDistance, out var vd) || vd is < 2 or > 32)
            yield return $"View Distance must be 2–32 (got '{_viewDistance}')";

        if (!IsValidMemory(_jvmMin))
            yield return $"JVM Min Memory must be a value like 512M or 2G (got '{_jvmMin}')";

        if (!IsValidMemory(_jvmMax))
            yield return $"JVM Max Memory must be a value like 512M or 2G (got '{_jvmMax}')";

        if (string.IsNullOrWhiteSpace(_worldName))
            yield return "World Name is required";

        if (_rconEnabled)
        {
            if (!TryParsePort(_rconPort))
                yield return $"RCON Port must be 1–65535 (got '{_rconPort}')";

            if (string.IsNullOrWhiteSpace(_rconPassword))
                yield return "RCON Password is required when RCON is enabled";

            if (!int.TryParse(_rconTimeout, out var rt) || rt < 1)
                yield return $"RCON Timeout must be a positive integer (got '{_rconTimeout}')";
        }
    }

    private static bool TryParsePort(string s) =>
        int.TryParse(s, out var v) && v is >= 1 and <= 65535;

    private static bool IsValidMemory(string s) =>
        Regex.IsMatch(s, @"^\d+[mMgG]$");

    // ─────────────────────────────────────────────────────────────────
    // Create
    // ─────────────────────────────────────────────────────────────────

    private void RunCreate()
    {
        var errors = Validate().ToList();
        if (errors.Count > 0)
        {
            _errorMessage = string.Join("\n", errors.Select(e => $"• {e}"));
            _step = Step.Error;
            return;
        }

        _step = Step.Creating;
        _progress = 0;
        _progressLabel = "Preparing…";
        _ = RunCreateAsync();
    }

    private async Task RunCreateAsync()
    {
        try
        {
            var server = new Server
            {
                Name = _name,
                Type = _type,
                Version = _selectedVersion!.Version,
                Network = new NetworkConfig
                {
                    Port = int.Parse(_port),
                    MaxPlayers = int.Parse(_maxPlayers),
                    MotD = _motd,
                    ViewDistance = int.Parse(_viewDistance),
                    OnlineMode = _onlineMode,
                    Whitelist = _whitelist,
                },
                Rcon = new RconConfig
                {
                    Enabled = _rconEnabled,
                    Port = int.Parse(_rconPort),
                    Password = _rconPassword,
                    TimeoutSeconds = int.Parse(_rconTimeout),
                },
                Jvm = new JvmConfig { MinMemory = _jvmMin, MaxMemory = _jvmMax },
                World = new WorldConfig
                {
                    Name = _worldName,
                    Seed = string.IsNullOrWhiteSpace(_worldSeed) ? null : _worldSeed,
                    GameMode = _gameMode,
                    Difficulty = _difficulty,
                },
            };

            await manager.CreateAsync(server, new ProgressRelay(p =>
            {
                _progress = p;
                _progressLabel = $"Downloading… {p:P0}";
            }));

            _step = Step.Done;
        }
        catch (Exception ex)
        {
            _errorMessage = ex.Message;
            _step = Step.Error;
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // Render
    // ─────────────────────────────────────────────────────────────────

    public override IRenderable Render()
    {
        if (_step == Step.Done)
        {
            navigator.Pop();
            return new Markup("");
        }

        EnsureLayout();

        return _step switch
        {
            Step.Creating => RenderCreating(),
            Step.Error => RenderFormLayout(_errorMessage),
            _ => RenderFormLayout(null),
        };
    }

    private void EnsureLayout()
    {
        if (_layout is not null) return;
        _layout = new Layout("Root")
            .SplitRows(
                new Layout("Main"),
                new Layout("Footer").Size(3)
            );
    }

    private IRenderable RenderFormLayout(string? error)
    {
        _layout!["Main"].Update(Align.Center(
            new Rows(
                RenderPageIndicator(),
                RenderCurrentTab()
            ),
            VerticalAlignment.Middle
        ));
        _layout["Footer"].Update(RenderHelp(error));
        return _layout;
    }

    private Markup RenderPageIndicator()
    {
        var parts = TabNames.Select((n, i) =>
            i == _tabIndex ? $"[bold green] {n} [/]" : $"[dim] {n} [/]");
        return new Markup(string.Join("[dim]|[/]", parts));
    }

    private IRenderable RenderCurrentTab()
    {
        const int sideW = 20;

        var windowSize = Math.Max(3, Console.WindowHeight - 10);
        var offset = _offsets[_tabIndex];
        var fields = GetTabFields();

        var formTable = new Table()
            .HideHeaders().NoBorder().Collapse()
            .AddColumn(new TableColumn("").NoWrap().RightAligned().Width(sideW))
            .AddColumn(new TableColumn("").NoWrap().Centered().Width(1))
            .AddColumn(new TableColumn("").NoWrap().LeftAligned().Width(sideW));

        var end = Math.Min(offset + windowSize, fields.Count);
        for (var i = offset; i < end; i++)
        {
            var f = fields[i];
            var sel = IsActive(f.Index);
            var prefix = sel ? "→ " : "  ";
            formTable.AddRow(
                new Markup(sel
                    ? $"[bold green reverse]{Markup.Escape(prefix + f.Label)}[/]"
                    : $"[white]{Markup.Escape(prefix + f.Label)}[/]"),
                new Markup("[dim]:[/]"),
                new Markup(sel
                    ? $"[bold green reverse]{f.Value}[/]"
                    : $"[green]{f.Value}[/]")
            );
        }

        var btnSel = IsActive(TabFieldCounts[_tabIndex] - 1);
        var content = new Table()
            .HideHeaders().NoBorder().Collapse()
            .AddColumn(new TableColumn("").NoWrap().Centered());
        content.AddRow(new Align(formTable, HorizontalAlignment.Center));
        content.AddRow(new Markup(""));
        content.AddRow(new Align(
            new Markup(btnSel
                ? "[bold green reverse][[ Create Server ]][/]"
                : "[green][[ Create Server ]][/]"
            ),
            HorizontalAlignment.Center)
        );

        return content;
    }

    private readonly record struct FieldEntry(int Index, string Label, string Value);

    private List<FieldEntry> GetTabFields() => _tabIndex switch
    {
        0 =>
        [
            new FieldEntry(0, "Name", TextField(_name, 0)),
            new FieldEntry(1, "Type", PlainPickerValue(_type.ToString())),
            new FieldEntry(2, "Version", PlainVersionValue()),
        ],
        1 =>
        [
            new FieldEntry(0, "Port", TextField(_port, 0)),
            new FieldEntry(1, "Max Players", TextField(_maxPlayers, 1)),
            new FieldEntry(2, "MotD", TextField(_motd, 2)),
            new FieldEntry(3, "View Distance", TextField(_viewDistance, 3)),
            new FieldEntry(4, "Online Mode", BoolValue(_onlineMode)),
            new FieldEntry(5, "Whitelist", BoolValue(_whitelist)),
        ],
        2 =>
        [
            new FieldEntry(0, "Enabled", BoolValue(_rconEnabled)),
            new FieldEntry(1, "Port", TextField(_rconPort, 1)),
            new FieldEntry(2, "Password", TextField(_rconPassword, 2)),
            new FieldEntry(3, "Timeout (s)", TextField(_rconTimeout, 3)),
        ],
        3 =>
        [
            new FieldEntry(0, "Min Memory", TextField(_jvmMin, 0)),
            new FieldEntry(1, "Max Memory", TextField(_jvmMax, 1)),
        ],
        _ =>
        [
            new FieldEntry(0, "World Name", TextField(_worldName, 0)),
            new FieldEntry(1, "Seed", TextField(_worldSeed, 1, "(random)")),
            new FieldEntry(2, "Game Mode", PlainPickerValue(_gameMode.ToString())),
            new FieldEntry(3, "Difficulty", PlainPickerValue(_difficulty.ToString())),
        ],
    };

    // ── Creating ──────────────────────────────────────────────────────

    private IRenderable RenderCreating()
    {
        const int barWidth = 40;
        var filled = (int)(_progress * barWidth);
        var bar = new string('█', filled) + new string('░', barWidth - filled);

        var content = new Rows(
            new Markup(_progressLabel),
            new Markup($"[green]{bar}[/] [bold]{_progress:P0}[/]")
        );

        _layout!["Header"].Update(new Align(
            new Markup($"[bold]Creating [green]{Markup.Escape(_name)}[/]…[/]"),
            HorizontalAlignment.Center, VerticalAlignment.Middle));
        _layout["Main"].Update(new Align(content, HorizontalAlignment.Center, VerticalAlignment.Middle));
        _layout["Footer"].Update(new Markup(""));
        return _layout;
    }

    // ─────────────────────────────────────────────────────────────────
    // Field value helpers
    // ─────────────────────────────────────────────────────────────────

    private bool IsActive(int index) => _fieldCursors[_tabIndex] == index;

    private string TextField(string value, int index, string placeholder = "")
    {
        var active = IsActive(index);
        var display = value.Length > 0
            ? Markup.Escape(value)
            : placeholder.Length > 0
                ? $"[dim]{Markup.Escape(placeholder)}[/]"
                : "";
        return active ? display + "[blink]_[/]" : display;
    }

    private static string BoolValue(bool on) =>
        on ? "[green]On[/]" : "[red]Off[/]";

    private static string PlainPickerValue(string value) =>
        Markup.Escape(value);

    private string PlainVersionValue()
    {
        if (_selectedVersion is null)
            return "[dim](none — press Enter)[/]";
        return Markup.Escape(_selectedVersion.Version)
               + (_selectedVersion.IsSnapshot ? " [dim](snapshot)[/]" : "");
    }

    private IRenderable RenderHelp(string? error)
    {
        const string hint = "[dim]↑↓/Tab:nav  ←→:page  Enter:activate  Space:toggle  Esc:cancel[/]";
        var dismiss = "[dim][b]Enter[/] or [b]Esc[/] to dismiss[/]";
        var text = string.IsNullOrWhiteSpace(error)
            ? hint
            : $"[bold red]{Markup.Escape(error)}[/]\n{dismiss}";
        return new Align(new Markup(text), HorizontalAlignment.Center, VerticalAlignment.Middle);
    }


    private StringRef? ActiveTextField() => (_tabIndex, _fieldCursors[_tabIndex]) switch
    {
        (0, 0) => Ref(() => _name, v => _name = v),
        (1, 0) => Ref(() => _port, v => _port = v),
        (1, 1) => Ref(() => _maxPlayers, v => _maxPlayers = v),
        (1, 2) => Ref(() => _motd, v => _motd = v),
        (1, 3) => Ref(() => _viewDistance, v => _viewDistance = v),
        (2, 1) => Ref(() => _rconPort, v => _rconPort = v),
        (2, 2) => Ref(() => _rconPassword, v => _rconPassword = v),
        (2, 3) => Ref(() => _rconTimeout, v => _rconTimeout = v),
        (3, 0) => Ref(() => _jvmMin, v => _jvmMin = v),
        (3, 1) => Ref(() => _jvmMax, v => _jvmMax = v),
        (4, 0) => Ref(() => _worldName, v => _worldName = v),
        (4, 1) => Ref(() => _worldSeed, v => _worldSeed = v),
        _ => null,
    };

    private static StringRef Ref(Func<string> get, Action<string> set) => new(get, set);

    private sealed class StringRef(Func<string> get, Action<string> set)
    {
        public string Value
        {
            get => get();
            set => set(value);
        }
    }

    private sealed class ProgressRelay(Action<double> onProgress) : IProgressCallback
    {
        public void OnProgress(double progress) => onProgress(progress);
    }
}