using Hestia.Core.Server;
using Hestia.Tui.Input;
using Hestia.Tui.Services;
using Spectre.Console;
using static Hestia.Tui.Utilities.ServerUtils;
using static Hestia.Tui.Utilities.TerminalUtils;

namespace Hestia.Tui.Screens.Modals;

internal static class CreateWizardModal
{
    private enum CreateMode
    {
        Normal,
        EditText,
        SelectVersion,
        SelectType,
        ConfirmEula
    }

    private enum CreatePage
    {
        Basic,
        Gameplay,
        Network,
        Java
    }

    private sealed class WizardState
    {
        public CreateMode Mode { get; set; } = CreateMode.Normal;
        public CreatePage Page { get; set; } = CreatePage.Basic;
        public bool EulaCursorYes { get; set; } = true;

        public string EditBuffer { get; set; } = string.Empty;
        public string EditOriginal { get; set; } = string.Empty;

        public string VersionQuery { get; set; } = string.Empty;
        public int VersionCursor { get; set; }
        public string VersionOriginal { get; set; } = string.Empty;

        public int TypeCursor { get; set; }
        public ServerType TypeOriginal { get; set; }

        public string Error { get; set; } = string.Empty;

        public IReadOnlyList<string> Versions { get; set; } = Array.Empty<string>();
    }

    private readonly record struct KeyResult(bool Dirty, CreateModalResult? ModalResult);

    private static KeyResult HandleKey(
        ServerCreateForm form,
        IReadOnlyDictionary<ServerType, IReadOnlyList<string>> versionsByType,
        WizardState state,
        ConsoleKeyInfo key,
        InputAction? action)
    {
        var visibleFields = GetVisibleFields(state.Page, form.RconEnabled);
        if (visibleFields.IndexOf(form.SelectedField) < 0)
            form.SelectedField = visibleFields.Count > 0 ? visibleFields[0] : ServerCreateForm.Field.Submit;

        if (action == InputAction.Escape)
            return HandleEscape(form, state)
                ? new KeyResult(true, null)
                : new KeyResult(true, new CreateModalResult(null));

        return state.Mode switch
        {
            CreateMode.Normal => HandleNormalKey(form, state, key, action, visibleFields),
            CreateMode.EditText => HandleEditTextKey(form, state, key, action, visibleFields),
            CreateMode.SelectVersion => HandleSelectVersionKey(form, state, key, action, visibleFields),
            CreateMode.SelectType => HandleSelectTypeKey(form, versionsByType, state, action, visibleFields),
            CreateMode.ConfirmEula => HandleConfirmEulaKey(form, state, action),
            _ => new KeyResult(false, null),
        };
    }

    private static KeyResult HandleNormalKey(
        ServerCreateForm form,
        WizardState state,
        ConsoleKeyInfo key,
        InputAction? action,
        IReadOnlyList<ServerCreateForm.Field> visibleFields)
    {
        if (action is InputAction.TabLeft or InputAction.TabRight)
        {
            state.Page = action == InputAction.TabLeft ? PrevPage(state.Page) : NextPage(state.Page);
            state.Error = string.Empty;
            return new KeyResult(true, null);
        }

        if (action == InputAction.CursorUp)
        {
            form.MoveUp(visibleFields);
            return new KeyResult(true, null);
        }

        if (action == InputAction.CursorDown)
        {
            form.MoveDown(visibleFields);
            return new KeyResult(true, null);
        }

        if (action is InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
        {
            if (action == InputAction.CycleFocusPrev) form.MoveUp(visibleFields);
            else form.MoveDown(visibleFields);
            state.Error = string.Empty;
            return new KeyResult(true, null);
        }

        if (key.Key == ConsoleKey.Spacebar)
            return HandleSpacebar(form, state);

        if (action is not (InputAction.Confirm or InputAction.OpenCommand))
            return new KeyResult(false, null);

        if (form.SelectedField == ServerCreateForm.Field.Submit)
            return HandleSubmit(form, state);

        if (form.SelectedField == ServerCreateForm.Field.Version)
            return BeginSelectVersion(form, state);

        if (form.SelectedField == ServerCreateForm.Field.Type)
            return BeginSelectType(form, state);

        if (form.IsTextEditable(form.SelectedField))
            return BeginEditText(form, state);
        if (form.SelectedField is ServerCreateForm.Field.OnlineMode
            or ServerCreateForm.Field.Whitelist
            or ServerCreateForm.Field.RconEnabled)
            return HandleSpacebar(form, state);

        state.Error = string.Empty;
        return new KeyResult(true, null);
    }

    private static KeyResult HandleSpacebar(ServerCreateForm form, WizardState state)
    {
        var toggled = form.SelectedField switch
        {
            ServerCreateForm.Field.OnlineMode => Toggle(form.ToggleOnlineMode),
            ServerCreateForm.Field.Whitelist => Toggle(form.ToggleWhitelist),
            ServerCreateForm.Field.RconEnabled => Toggle(form.ToggleRconEnabled),
            _ => false,
        };

        if (!toggled) return new KeyResult(false, null);
        state.Error = string.Empty;
        return new KeyResult(true, null);
    }

    private static KeyResult HandleSubmit(ServerCreateForm form, WizardState state)
    {
        state.Error = ValidateForSubmit(form) ?? string.Empty;
        if (!string.IsNullOrWhiteSpace(state.Error))
            return new KeyResult(true, null);

        state.EulaCursorYes = true;
        state.Mode = CreateMode.ConfirmEula;
        return new KeyResult(true, null);
    }

    private static KeyResult BeginSelectVersion(ServerCreateForm form, WizardState state)
    {
        state.VersionOriginal = form.Version;
        state.VersionQuery = string.Empty;
        state.VersionCursor = Math.Max(0, FindIndex(state.Versions, form.Version));
        state.Mode = CreateMode.SelectVersion;
        state.Error = string.Empty;
        return new KeyResult(true, null);
    }

    private static KeyResult BeginSelectType(ServerCreateForm form, WizardState state)
    {
        state.TypeOriginal = form.Type;
        state.TypeCursor = Math.Max(0, Array.IndexOf(form.Types, form.Type));
        state.Mode = CreateMode.SelectType;
        state.Error = string.Empty;
        return new KeyResult(true, null);
    }

    private static KeyResult BeginEditText(ServerCreateForm form, WizardState state)
    {
        state.EditOriginal = form.GetTextValue(form.SelectedField);
        state.EditBuffer = state.EditOriginal;
        state.Mode = CreateMode.EditText;
        state.Error = string.Empty;
        return new KeyResult(true, null);
    }

    private static KeyResult HandleEditTextKey(
        ServerCreateForm form,
        WizardState state,
        ConsoleKeyInfo key,
        InputAction? action,
        IReadOnlyList<ServerCreateForm.Field> visibleFields)
    {
        if (action == InputAction.TextBackspace)
        {
            if (state.EditBuffer.Length > 0) state.EditBuffer = state.EditBuffer[..^1];
            return new KeyResult(true, null);
        }

        if (action is InputAction.Confirm or InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
        {
            if (!TryApplyEdit(form, form.SelectedField, state.EditBuffer, out var error))
            {
                state.Error = error;
                return new KeyResult(true, null);
            }

            state.EditBuffer = string.Empty;
            state.Mode = CreateMode.Normal;
            state.Error = string.Empty;
            MoveAfterConfirm(action, form, visibleFields);
            return new KeyResult(true, null);
        }

        if (char.IsControl(key.KeyChar)) return new KeyResult(false, null);
        state.EditBuffer += key.KeyChar;
        return new KeyResult(true, null);
    }

    private static KeyResult HandleSelectVersionKey(
        ServerCreateForm form,
        WizardState state,
        ConsoleKeyInfo key,
        InputAction? action,
        IReadOnlyList<ServerCreateForm.Field> visibleFields)
    {
        var filtered = FilterVersions(state.Versions, state.VersionQuery);
        state.VersionCursor = filtered.Count == 0 ? 0 : Math.Clamp(state.VersionCursor, 0, filtered.Count - 1);

        if (action == InputAction.CursorUp)
        {
            if (state.VersionCursor > 0) state.VersionCursor--;
            return new KeyResult(true, null);
        }

        if (action == InputAction.CursorDown)
        {
            if (filtered.Count > 0 && state.VersionCursor < filtered.Count - 1) state.VersionCursor++;
            return new KeyResult(true, null);
        }

        if (action == InputAction.TextBackspace)
        {
            if (state.VersionQuery.Length > 0) state.VersionQuery = state.VersionQuery[..^1];
            state.VersionCursor = 0;
            return new KeyResult(true, null);
        }

        if (action is InputAction.Confirm or InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
        {
            if (filtered.Count > 0) form.Version = filtered[state.VersionCursor];
            state.VersionQuery = string.Empty;
            state.VersionCursor = 0;
            state.Mode = CreateMode.Normal;
            state.Error = string.Empty;
            MoveAfterConfirm(action, form, visibleFields);
            return new KeyResult(true, null);
        }

        if (action == InputAction.OpenCommand || char.IsControl(key.KeyChar))
            return new KeyResult(false, null);

        state.VersionQuery += key.KeyChar;
        state.VersionCursor = 0;
        return new KeyResult(true, null);
    }

    private static KeyResult HandleSelectTypeKey(
        ServerCreateForm form,
        IReadOnlyDictionary<ServerType, IReadOnlyList<string>> versionsByType,
        WizardState state,
        InputAction? action,
        IReadOnlyList<ServerCreateForm.Field> visibleFields)
    {
        var types = form.Types;
        state.TypeCursor = Math.Clamp(state.TypeCursor, 0, Math.Max(0, types.Length - 1));

        if (action == InputAction.CursorUp)
        {
            if (state.TypeCursor > 0) state.TypeCursor--;
            return new KeyResult(true, null);
        }

        if (action == InputAction.CursorDown)
        {
            if (types.Length > 0 && state.TypeCursor < types.Length - 1) state.TypeCursor++;
            return new KeyResult(true, null);
        }

        if (action is not (InputAction.Confirm or InputAction.CycleFocusNext or InputAction.CycleFocusPrev))
            return new KeyResult(false, null);

        if (types.Length > 0)
        {
            form.Type = types[state.TypeCursor];
            state.Versions = versionsByType.TryGetValue(form.Type, out var tv) ? tv : Array.Empty<string>();
            state.VersionQuery = string.Empty;
            state.VersionCursor = FindIndex(state.Versions, form.Version);
            if (state.VersionCursor < 0) state.VersionCursor = 0;
        }

        state.Mode = CreateMode.Normal;
        state.Error = string.Empty;
        MoveAfterConfirm(action, form, visibleFields);
        return new KeyResult(true, null);
    }

    private static KeyResult HandleConfirmEulaKey(ServerCreateForm form, WizardState state, InputAction? action)
    {
        if (action is InputAction.TabLeft or InputAction.TabRight or InputAction.CursorUp or InputAction.CursorDown)
        {
            state.EulaCursorYes = !state.EulaCursorYes;
            return new KeyResult(true, null);
        }

        if (action != InputAction.Confirm) return new KeyResult(false, null);

        form.AcceptEula = state.EulaCursorYes;
        if (!form.AcceptEula)
        {
            state.Error = "You must accept the EULA to create";
            state.Mode = CreateMode.Normal;
            return new KeyResult(true, null);
        }

        state.Error = string.Empty;
        return new KeyResult(true, new CreateModalResult(form));
    }

    private static bool HandleEscape(ServerCreateForm form, WizardState state)
    {
        switch (state.Mode)
        {
            case CreateMode.EditText:
                state.EditBuffer = state.EditOriginal;
                state.Mode = CreateMode.Normal;
                state.Error = string.Empty;
                return true;
            case CreateMode.SelectVersion:
                form.Version = state.VersionOriginal;
                state.VersionQuery = string.Empty;
                state.VersionCursor = 0;
                state.Mode = CreateMode.Normal;
                state.Error = string.Empty;
                return true;
            case CreateMode.SelectType:
                form.Type = state.TypeOriginal;
                state.TypeCursor = 0;
                state.Mode = CreateMode.Normal;
                state.Error = string.Empty;
                return true;
            case CreateMode.ConfirmEula:
                state.Mode = CreateMode.Normal;
                state.Error = string.Empty;
                return true;
            default:
                return false;
        }
    }

    private static void MoveAfterConfirm(InputAction? action, ServerCreateForm form,
        IReadOnlyList<ServerCreateForm.Field> visibleFields)
    {
        if (action == InputAction.CycleFocusPrev) form.MoveUp(visibleFields);
        else if (action == InputAction.CycleFocusNext) form.MoveDown(visibleFields);
    }

    private static string? ValidateForSubmit(ServerCreateForm form)
    {
        if (string.IsNullOrWhiteSpace(form.Name)) return "Server name required";
        if (!IsValidPort(form.ServerPort)) return "Server port must be 1-65535";
        if (form.MaxPlayers is < 1 or > 10_000) return "Max players must be 1-10000";
        if (form.ViewDistance is < 2 or > 32) return "View distance must be 2-32";
        if (string.IsNullOrWhiteSpace(form.LevelName)) return "Level name required";
        if (string.IsNullOrWhiteSpace(form.Difficulty)) return "Difficulty required";
        if (!form.RconEnabled) return null;
        if (!IsValidPort(form.RconPort)) return "RCON port must be 1-65535";
        if (form.ServerPort == form.RconPort) return "Server port and RCON port must differ";
        if (string.IsNullOrWhiteSpace(form.RconPassword)) return "RCON password required";
        return form.RconTimeoutSeconds is < 1 or > 120 ? "RCON timeout must be 1-120" : null;
    }

    private static bool TryApplyEdit(ServerCreateForm form, ServerCreateForm.Field field, string buf, out string error)
    {
        error = string.Empty;
        switch (field)
        {
            case ServerCreateForm.Field.Name:
                form.SetName(buf);
                return true;
            case ServerCreateForm.Field.Directory:
                form.SetDirectory(buf);
                return true;
            case ServerCreateForm.Field.MotD:
                form.SetMotD(buf);
                return true;
            case ServerCreateForm.Field.LevelName:
                form.SetLevelName(buf);
                return true;
            case ServerCreateForm.Field.Difficulty:
                form.SetDifficulty(buf);
                return true;
            case ServerCreateForm.Field.RconPassword:
                form.SetRconPassword(buf);
                return true;
            case ServerCreateForm.Field.JvmMinMemory:
                form.SetJvmMinMemory(buf);
                return true;
            case ServerCreateForm.Field.JvmMaxMemory:
                form.SetJvmMaxMemory(buf);
                return true;
            case ServerCreateForm.Field.JvmAdditionalFlags:
                form.JvmAdditionalFlags = buf;
                return true;

            case ServerCreateForm.Field.ServerPort:
                if (!TryParsePort(buf, out var sp))
                {
                    error = "Server port must be 1-65535";
                    return false;
                }

                form.ServerPort = sp;
                return true;

            case ServerCreateForm.Field.MaxPlayers:
                if (!int.TryParse(buf.Trim(), out var mp) || mp is < 1 or > 10_000)
                {
                    error = "Max players must be 1-10000";
                    return false;
                }

                form.MaxPlayers = mp;
                return true;

            case ServerCreateForm.Field.ViewDistance:
                if (!int.TryParse(buf.Trim(), out var vd) || vd is < 2 or > 32)
                {
                    error = "View distance must be 2-32";
                    return false;
                }

                form.ViewDistance = vd;
                return true;

            case ServerCreateForm.Field.RconPort:
                if (!TryParsePort(buf, out var rp))
                {
                    error = "RCON port must be 1-65535";
                    return false;
                }

                form.RconPort = rp;
                return true;

            case ServerCreateForm.Field.RconTimeoutSeconds:
                if (!int.TryParse(buf.Trim(), out var rt) || rt is < 1 or > 120)
                {
                    error = "RCON timeout must be 1-120";
                    return false;
                }

                form.RconTimeoutSeconds = rt;
                return true;

            default:
                return true;
        }
    }

    public static async Task<CreateModalResult> RunAsync(
        ServerCreateForm form,
        IReadOnlyDictionary<ServerType, IReadOnlyList<string>> versionsByType,
        KeyMap keyMap,
        CancellationToken ct)
    {
        Console.CursorVisible = false;

        var state = new WizardState
        {
            Versions = versionsByType.TryGetValue(form.Type, out var initVersions)
                ? initVersions
                : Array.Empty<string>(),
        };

        try
        {
            while (!ct.IsCancellationRequested)
            {
                while (TooSmall() && !ct.IsCancellationRequested)
                {
                    AnsiConsole.Clear();
                    Console.WriteLine($"Terminal too small. Resize to at least {MinWidth}×{MinHeight}.");
                    await Task.Delay(300, ct);
                }

                if (ct.IsCancellationRequested) break;

                CreateModalResult? result = null;
                var dirty = true;

                var layout = new Layout()
                    .SplitRows(
                        new Layout("Form"),
                        new Layout("Help").Size(3)
                    );

                await AnsiConsole.Live(layout)
                    .AutoClear(false)
                    .Overflow(VerticalOverflow.Ellipsis)
                    .Cropping(VerticalOverflowCropping.Bottom)
                    .StartAsync(async ctx =>
                    {
                        while (!ct.IsCancellationRequested && result is null)
                        {
                            if (TooSmall()) return;

                            if (dirty)
                            {
                                UpdateLayout(layout, form, state);
                                ctx.Refresh();
                                dirty = false;
                            }

                            if (!Console.KeyAvailable)
                            {
                                await Task.Delay(50, ct);
                                continue;
                            }

                            var key = Console.ReadKey(true);
                            var routed = HandleKey(form, versionsByType, state, key, keyMap.Translate(key));

                            if (routed.ModalResult is not null)
                            {
                                result = routed.ModalResult;
                                return;
                            }

                            if (routed.Dirty) dirty = true;
                        }
                    });

                if (result is not null) return result;
            }
        }
        catch (OperationCanceledException) { }
        finally
        {
            Console.CursorVisible = false;
            AnsiConsole.Clear();
        }

        return new CreateModalResult(null);
    }

    private static void UpdateLayout(Layout layout, ServerCreateForm form, WizardState state)
    {
        var visibleFields = GetVisibleFields(state.Page, form.RconEnabled);
        if (visibleFields.IndexOf(form.SelectedField) < 0)
            form.SelectedField = visibleFields.Count > 0 ? visibleFields[0] : ServerCreateForm.Field.Submit;

        var pageIndicator = GetPageIndicator(state.Page);

        var help = state.Mode switch
        {
            CreateMode.Normal => "[dim]↑↓/Tab:nav  ←→:page  Enter:activate  Space:toggle  Esc:cancel[/]",
            CreateMode.EditText =>
                "[dim]Type to edit  Enter:confirm  Esc:cancel  Tab:confirm+next  Backspace:delete[/]",
            CreateMode.SelectVersion =>
                "[dim]↑↓:select  Type:search  Enter:confirm  Esc:cancel  Tab:confirm+next  Backspace:delete[/]",
            CreateMode.SelectType => "[dim]↑↓:select  Enter:confirm  Esc:cancel  Tab:confirm+next[/]",
            CreateMode.ConfirmEula => "[dim]←→:choose  Enter:confirm  Esc:back[/]",
            _ => "[dim][/]",
        };

        var helpMarkup = string.IsNullOrWhiteSpace(state.Error)
            ? $"\n{help}"
            : $"[bold red]{Markup.Escape(state.Error)}[/]\n{help}";

        var pageIndicatorMarkup = new Markup($"[dim]{pageIndicator}[/]");
        var formContent = RenderFormContent(form, state, visibleFields);

        var container = new Table()
            .HideHeaders().NoBorder().Collapse()
            .AddColumn(new TableColumn(string.Empty).NoWrap().Centered());
        container.AddRow(pageIndicatorMarkup);
        container.AddRow(formContent);

        var formPanel = new Panel(container)
        {
            Expand = false,
            Border = BoxBorder.None,
        };

        layout["Form"].Update(new Align(formPanel, HorizontalAlignment.Center, VerticalAlignment.Middle));
        layout["Help"].Update(new Align(new Markup(helpMarkup), HorizontalAlignment.Center));
    }

    private static Table RenderFormContent(
        ServerCreateForm form,
        WizardState state,
        IReadOnlyList<ServerCreateForm.Field> visibleFields)
    {
        const int FormPadding = 4;
        var muted = state.Mode is CreateMode.SelectVersion or CreateMode.SelectType;

        var fields = new List<(string Label, ServerCreateForm.Field Field)>(visibleFields.Count);
        foreach (var f in visibleFields)
        {
            if (f != ServerCreateForm.Field.Submit)
                fields.Add((FieldLabel(f), f));
        }

        var (totalW, leftW, rightW) = ComputeWidths(form, state, fields, FormPadding);

        var formTable = new Table()
            .HideHeaders().NoBorder().Collapse()
            .AddColumn(new TableColumn(string.Empty).NoWrap().RightAligned())
            .AddColumn(new TableColumn(string.Empty).NoWrap().Centered())
            .AddColumn(new TableColumn(string.Empty).NoWrap().LeftAligned());

        formTable.Columns[0].Width(leftW);
        formTable.Columns[1].Width(1);
        formTable.Columns[2].Width(rightW);

        foreach (var (label, f) in fields)
        {
            var selected = IsFieldSelected(form, state, f);
            var labelStyle = selected ? "bold yellow reverse" : muted ? "dim" : "white";
            var valueStyle = selected ? "bold cyan reverse" : muted ? "dim" : "cyan";
            var prefix = selected ? "→ " : "  ";
            formTable.AddRow(
                new Markup($"[{labelStyle}]{Markup.Escape(prefix + label)}[/]"),
                new Markup("[dim]:[/]"),
                new Markup($"[{valueStyle}]{RichValue(form, state, f)}[/]"));
        }

        var content = new Table()
            .HideHeaders().NoBorder().Collapse()
            .AddColumn(new TableColumn(string.Empty).NoWrap().Centered());

        content.AddRow(new Align(formTable, HorizontalAlignment.Center));

        if (state.Mode == CreateMode.SelectVersion)
            AppendVersionPicker(content, state, totalW);
        else if (state.Mode == CreateMode.SelectType)
            AppendTypePicker(content, form, state, totalW);

        if (state.Mode is CreateMode.Normal or CreateMode.EditText)
            AppendSubmitButton(content, form, state);

        if (state.Mode == CreateMode.ConfirmEula)
            AppendEulaConfirm(content, state, totalW);

        return content;
    }

    private static (int TotalW, int LeftW, int RightW) ComputeWidths(
        ServerCreateForm form,
        WizardState state,
        List<(string Label, ServerCreateForm.Field Field)> fields,
        int padding)
    {
        var availableW = Math.Max(1, Console.WindowWidth - padding);
        var seamTextW = 0;
        var valueTextW = 0;

        foreach (var (label, f) in fields)
        {
            seamTextW = Math.Max(seamTextW, (IsFieldSelected(form, state, f) ? "→ " : "  ").Length + label.Length);
            valueTextW = Math.Max(valueTextW, PlainValue(form, state, f).Length);
        }

        valueTextW = Math.Max(valueTextW, "[ Create Server ]".Length);

        if (state.Mode == CreateMode.ConfirmEula)
        {
            valueTextW = Math.Max(valueTextW, "Accept Minecraft EULA?".Length);
            valueTextW = Math.Max(valueTextW, "https://aka.ms/MinecraftEULA".Length);
            valueTextW = Math.Max(valueTextW, "  YES     NO  ".Length);
        }
        else if (state.Mode == CreateMode.SelectVersion)
        {
            var filtered = FilterVersions(state.Versions, state.VersionQuery);
            var (start, end, cur) = VersionPage(filtered.Count, state.VersionCursor);
            valueTextW = Math.Max(valueTextW, "...".Length);
            for (var i = start; i < end; i++)
                valueTextW = Math.Max(valueTextW, (i == cur ? "→ " : "  ").Length + filtered[i].Length);
            valueTextW = Math.Max(valueTextW, $"Search: {state.VersionQuery}█".Length);
        }
        else if (state.Mode == CreateMode.SelectType)
        {
            var types = form.Types;
            var cur = Math.Clamp(state.TypeCursor, 0, Math.Max(0, types.Length - 1));
            for (var i = 0; i < types.Length; i++)
                valueTextW = Math.Max(valueTextW, (i == cur ? "→ " : "  ").Length + types[i].ToString().Length);
        }

        var desiredW = Math.Max(17, (2 * Math.Max(seamTextW, valueTextW)) + 1);
        var totalW = Math.Min(availableW, desiredW);
        if (totalW % 2 == 0)
            totalW = totalW < availableW ? totalW + 1 : totalW - 1;

        var sideW = Math.Max(1, (totalW - 1) / 2);
        return (totalW, sideW, totalW - 1 - sideW);
    }

    private static void AppendVersionPicker(Table content, WizardState state, int totalW)
    {
        content.AddRow(new Align(new Markup($"[dim]{new string('─', totalW)}[/]"), HorizontalAlignment.Center));

        var list = new Table()
            .HideHeaders().NoBorder().Collapse()
            .AddColumn(new TableColumn(string.Empty).NoWrap().Centered().Width(totalW));

        var filtered = FilterVersions(state.Versions, state.VersionQuery);
        var (start, end, cur) = VersionPage(filtered.Count, state.VersionCursor);

        if (start > 0) list.AddRow(new Markup("[dim]...[/]"));

        for (var i = start; i < end; i++)
        {
            var sel = i == cur;
            list.AddRow(new Markup(
                $"[{(sel ? "bold cyan reverse" : "white")}]{(sel ? "→ " : "  ")}{Markup.Escape(filtered[i])}[/]"));
        }

        if (end < filtered.Count) list.AddRow(new Markup("[dim]...[/]"));
        list.AddRow(new Markup($"[dim]Search:[/] [bold]{Markup.Escape(state.VersionQuery)}[/][dim]█[/]"));

        content.AddRow(new Align(list, HorizontalAlignment.Center));
    }

    private static void AppendTypePicker(Table content, ServerCreateForm form, WizardState state, int totalW)
    {
        content.AddRow(new Align(new Markup($"[dim]{new string('─', totalW)}[/]"), HorizontalAlignment.Center));

        var list = new Table()
            .HideHeaders().NoBorder().Collapse()
            .AddColumn(new TableColumn(string.Empty).NoWrap().Centered().Width(totalW));

        var types = form.Types;
        var cur = Math.Clamp(state.TypeCursor, 0, Math.Max(0, types.Length - 1));
        for (var i = 0; i < types.Length; i++)
        {
            var sel = i == cur;
            list.AddRow(new Markup(
                $"[{(sel ? "bold cyan reverse" : "white")}]{(sel ? "→ " : "  ")}{Markup.Escape(types[i].ToString())}[/]"));
        }

        content.AddRow(new Align(list, HorizontalAlignment.Center));
    }

    private static void AppendSubmitButton(Table content, ServerCreateForm form, WizardState state)
    {
        var btnSelected = form.SelectedField == ServerCreateForm.Field.Submit && state.Mode == CreateMode.Normal;
        content.AddRow(new Markup(string.Empty));
        content.AddRow(new Align(
            new Markup($"[{(btnSelected ? "bold green reverse" : "green")}]{Markup.Escape("[ Create Server ]")}[/]"),
            HorizontalAlignment.Center));
    }

    private static void AppendEulaConfirm(Table content, WizardState state, int totalW)
    {
        content.AddRow(new Markup(string.Empty));
        content.AddRow(new Align(new Markup($"[dim]{new string('─', totalW)}[/]"), HorizontalAlignment.Center));

        var yesStyle = state.EulaCursorYes ? "bold green reverse" : "green";
        var noStyle = !state.EulaCursorYes ? "bold red reverse" : "red";

        var prompt = new Table()
            .HideHeaders().NoBorder().Collapse()
            .AddColumn(new TableColumn(string.Empty).NoWrap().Centered().Width(totalW));

        prompt.AddRow(new Markup("[bold]Accept Minecraft EULA?[/]"));
        prompt.AddRow(new Markup("[dim]https://aka.ms/MinecraftEULA[/]"));
        prompt.AddRow(new Markup(string.Empty));
        prompt.AddRow(new Markup($"[{yesStyle}]  YES  [/]   [{noStyle}]  NO  [/]"));

        content.AddRow(new Align(prompt, HorizontalAlignment.Center));
    }

    private static bool IsFieldSelected(ServerCreateForm form, WizardState state, ServerCreateForm.Field f) =>
        form.SelectedField == f && state.Mode is CreateMode.Normal or CreateMode.EditText;

    private static string FieldLabel(ServerCreateForm.Field field) => field switch
    {
        ServerCreateForm.Field.Name => "Name",
        ServerCreateForm.Field.Type => "Type",
        ServerCreateForm.Field.Version => "Version",
        ServerCreateForm.Field.ServerPort => "Server Port",
        ServerCreateForm.Field.MaxPlayers => "Max Players",
        ServerCreateForm.Field.MotD => "MotD",
        ServerCreateForm.Field.ViewDistance => "View Dist",
        ServerCreateForm.Field.OnlineMode => "Online",
        ServerCreateForm.Field.Whitelist => "Whitelist",
        ServerCreateForm.Field.LevelName => "Level",
        ServerCreateForm.Field.Difficulty => "Difficulty",
        ServerCreateForm.Field.Directory => "Directory",
        ServerCreateForm.Field.RconEnabled => "RCON",
        ServerCreateForm.Field.RconPort => "RCON Port",
        ServerCreateForm.Field.RconPassword => "RCON Pass",
        ServerCreateForm.Field.RconTimeoutSeconds => "RCON T/O",
        ServerCreateForm.Field.JvmMinMemory => "Xms",
        ServerCreateForm.Field.JvmMaxMemory => "Xmx",
        ServerCreateForm.Field.JvmAdditionalFlags => "JVM Flags",
        _ => string.Empty,
    };
    private static string PlainValue(ServerCreateForm form, WizardState state, ServerCreateForm.Field field)
    {
        if (state.Mode == CreateMode.EditText && form.SelectedField == field)
            return state.EditBuffer + "█";

        return field switch
        {
            ServerCreateForm.Field.Name => form.Name ?? string.Empty,
            ServerCreateForm.Field.Type => form.Type.ToString(),
            ServerCreateForm.Field.Version => form.Version ?? string.Empty,
            ServerCreateForm.Field.Directory => form.Directory ?? string.Empty,
            ServerCreateForm.Field.ServerPort => form.ServerPort.ToString(),
            ServerCreateForm.Field.MaxPlayers => form.MaxPlayers.ToString(),
            ServerCreateForm.Field.MotD => form.MotD ?? string.Empty,
            ServerCreateForm.Field.ViewDistance => form.ViewDistance.ToString(),
            ServerCreateForm.Field.OnlineMode => form.OnlineMode ? "ON" : "OFF",
            ServerCreateForm.Field.Whitelist => form.Whitelist ? "ON" : "OFF",
            ServerCreateForm.Field.LevelName => form.LevelName ?? string.Empty,
            ServerCreateForm.Field.Difficulty => form.Difficulty ?? string.Empty,
            ServerCreateForm.Field.RconEnabled => form.RconEnabled ? "ON" : "OFF",
            ServerCreateForm.Field.RconPort => form.RconPort.ToString(),
            ServerCreateForm.Field.RconPassword => form.RconPassword ?? string.Empty,
            ServerCreateForm.Field.RconTimeoutSeconds => form.RconTimeoutSeconds.ToString(),
            ServerCreateForm.Field.JvmMinMemory => form.JvmMinMemory ?? string.Empty,
            ServerCreateForm.Field.JvmMaxMemory => form.JvmMaxMemory ?? string.Empty,
            ServerCreateForm.Field.JvmAdditionalFlags => form.JvmAdditionalFlags ?? string.Empty,
            _ => string.Empty,
        };
    }
    private static string RichValue(ServerCreateForm form, WizardState state, ServerCreateForm.Field field)
    {
        if (state.Mode == CreateMode.EditText && form.SelectedField == field)
            return Markup.Escape(state.EditBuffer) + "[dim]█[/]";

        return field switch
        {
            ServerCreateForm.Field.OnlineMode => form.OnlineMode ? "[green]ON[/]" : "[red]OFF[/]",
            ServerCreateForm.Field.Whitelist => form.Whitelist ? "[green]ON[/]" : "[red]OFF[/]",
            ServerCreateForm.Field.RconEnabled => form.RconEnabled ? "[green]ON[/]" : "[red]OFF[/]",
            _ => Markup.Escape(PlainValue(form, state, field)),
        };
    }

    private static IReadOnlyList<ServerCreateForm.Field> GetVisibleFields(CreatePage page, bool rconEnabled)
    {
        List<ServerCreateForm.Field> fields = page switch
        {
            CreatePage.Basic => new List<ServerCreateForm.Field>
            {
                ServerCreateForm.Field.Name,
                ServerCreateForm.Field.Type,
                ServerCreateForm.Field.Version,
                ServerCreateForm.Field.ServerPort,
                ServerCreateForm.Field.MaxPlayers,
            },
            CreatePage.Gameplay => new List<ServerCreateForm.Field>
            {
                ServerCreateForm.Field.MotD,
                ServerCreateForm.Field.ViewDistance,
                ServerCreateForm.Field.OnlineMode,
                ServerCreateForm.Field.Whitelist,
                ServerCreateForm.Field.LevelName,
                ServerCreateForm.Field.Difficulty,
            },
            CreatePage.Network => BuildNetworkFields(rconEnabled),
            CreatePage.Java => new List<ServerCreateForm.Field>
            {
                ServerCreateForm.Field.JvmMinMemory,
                ServerCreateForm.Field.JvmMaxMemory,
                ServerCreateForm.Field.JvmAdditionalFlags,
            },
            _ => new List<ServerCreateForm.Field>(),
        };

        fields.Add(ServerCreateForm.Field.Submit);
        return fields;
    }

    private static List<ServerCreateForm.Field> BuildNetworkFields(bool rconEnabled)
    {
        var fields = new List<ServerCreateForm.Field>
        {
            ServerCreateForm.Field.Directory,
            ServerCreateForm.Field.RconEnabled,
        };

        if (rconEnabled)
        {
            fields.Add(ServerCreateForm.Field.RconPort);
            fields.Add(ServerCreateForm.Field.RconPassword);
            fields.Add(ServerCreateForm.Field.RconTimeoutSeconds);
        }

        return fields;
    }

    private static CreatePage NextPage(CreatePage page) => page switch
    {
        CreatePage.Basic => CreatePage.Gameplay,
        CreatePage.Gameplay => CreatePage.Network,
        CreatePage.Network => CreatePage.Java,
        _ => CreatePage.Basic,
    };

    private static CreatePage PrevPage(CreatePage page) => page switch
    {
        CreatePage.Basic => CreatePage.Java,
        CreatePage.Gameplay => CreatePage.Basic,
        CreatePage.Network => CreatePage.Gameplay,
        CreatePage.Java => CreatePage.Network,
        _ => CreatePage.Basic,
    };

    private static string GetPageIndicator(CreatePage page)
    {
        var pages = new[] { CreatePage.Basic, CreatePage.Gameplay, CreatePage.Network, CreatePage.Java };
        var names = new[] { "Basic", "Gameplay", "Network", "Java" };
        var current = Array.IndexOf(pages, page);

        var parts = new List<string>(pages.Length);
        for (int i = 0; i < pages.Length; i++)
        {
            if (i == current)
                parts.Add($"[bold cyan]{names[i]}[/]");
            else
                parts.Add(names[i]);
        }

        return string.Join("  »  ", parts);
    }

    private static (int Start, int End, int Cur) VersionPage(int count, int cursor)
    {
        const int PageSize = 8;
        var cur = count == 0 ? 0 : Math.Clamp(cursor, 0, count - 1);
        var start = Math.Max(0, cur - (PageSize / 2));
        var end = Math.Min(count, start + PageSize);
        if (end - start < PageSize && start > 0) start = Math.Max(0, end - PageSize);
        return (start, end, cur);
    }

    private static bool Toggle(Action a)
    {
        a();
        return true;
    }
}