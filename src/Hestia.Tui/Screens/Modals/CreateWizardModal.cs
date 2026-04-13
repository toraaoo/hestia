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
        ConfirmEula,
    }

    private enum CreatePage
    {
        Basic,
        Gameplay,
        Network,
        Java,
    }

    public static async Task<CreateModalResult> RunAsync(
        ServerCreateForm form,
        IReadOnlyDictionary<ServerType, IReadOnlyList<string>> versionsByType,
        KeyMap keyMap,
        CancellationToken ct)
    {
        Console.CursorVisible = false;

        var versions = versionsByType.TryGetValue(form.Type, out var v)
            ? v
            : Array.Empty<string>();

        // State
        var mode = CreateMode.Normal;
        var page = CreatePage.Basic;
        var advanced = false;

        var eulaCursorYes = true;

        var editBuffer = string.Empty;
        var editOriginal = string.Empty;

        var versionQuery = string.Empty;
        var versionCursor = 0;
        var versionOriginal = form.Version;

        var typeCursor = 0;
        var typeOriginal = form.Type;

        var createError = string.Empty;

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

                if (ct.IsCancellationRequested)
                    break;

                CreateModalResult? result = null;
                var dirty = true;

                var layout = new Layout()
                    .SplitRows(
                        new Layout("Form"),
                        // Fixed size to avoid rebuilding layout when showing/hiding error.
                        new Layout("Help").Size(3));

                await AnsiConsole.Live(layout)
                    .AutoClear(false)
                    .Overflow(VerticalOverflow.Ellipsis)
                    .Cropping(VerticalOverflowCropping.Bottom)
                    .StartAsync(async ctx =>
                    {
                        while (!ct.IsCancellationRequested && result is null)
                        {
                            if (TooSmall())
                                return;

                            if (dirty)
                            {
                                UpdateLayout(
                                    layout,
                                    form,
                                    versions,
                                    mode,
                                    page,
                                    advanced,
                                    editBuffer,
                                    editOriginal,
                                    ref versionQuery,
                                    versionCursor,
                                    typeCursor,
                                    eulaCursorYes,
                                    createError);
                                ctx.Refresh();
                                dirty = false;
                            }

                            if (!Console.KeyAvailable)
                            {
                                await Task.Delay(50, ct);
                                continue;
                            }

                            var key = Console.ReadKey(true);
                            var createAction = keyMap.Translate(key);

                            // Key handling is intentionally the same logic as before; the difference is we only re-render when dirty.
                            if (mode == CreateMode.Normal && key.Key == ConsoleKey.A)
                            {
                                advanced = !advanced;
                                if (!advanced)
                                    page = CreatePage.Basic;
                                createError = string.Empty;
                                dirty = true;
                                continue;
                            }

                            if (createAction == InputAction.Escape)
                            {
                                if (mode == CreateMode.EditText)
                                {
                                    editBuffer = editOriginal;
                                    mode = CreateMode.Normal;
                                    createError = string.Empty;
                                    dirty = true;
                                    continue;
                                }

                                if (mode == CreateMode.SelectVersion)
                                {
                                    form.Version = versionOriginal;
                                    versionQuery = string.Empty;
                                    versionCursor = 0;
                                    mode = CreateMode.Normal;
                                    createError = string.Empty;
                                    dirty = true;
                                    continue;
                                }

                                if (mode == CreateMode.SelectType)
                                {
                                    form.Type = typeOriginal;
                                    typeCursor = 0;
                                    mode = CreateMode.Normal;
                                    createError = string.Empty;
                                    dirty = true;
                                    continue;
                                }

                                if (mode == CreateMode.ConfirmEula)
                                {
                                    mode = CreateMode.Normal;
                                    createError = string.Empty;
                                    dirty = true;
                                    continue;
                                }

                                result = new CreateModalResult(null);
                                return;
                            }

                            var visibleFields = GetCreateVisibleFields(page, advanced, form.RconEnabled);
                            if (visibleFields.IndexOf(form.SelectedField) < 0)
                                form.SelectedField = visibleFields.Count > 0
                                    ? visibleFields[0]
                                    : ServerCreateForm.Field.Submit;

                            if (mode == CreateMode.Normal)
                            {
                                if (advanced && createAction is InputAction.TabLeft or InputAction.TabRight)
                                {
                                    page = createAction == InputAction.TabLeft
                                        ? PrevCreatePage(page)
                                        : NextCreatePage(page);
                                    createError = string.Empty;
                                    dirty = true;
                                    continue;
                                }

                                if (createAction == InputAction.CursorUp)
                                {
                                    form.MoveUp(visibleFields);
                                    dirty = true;
                                    continue;
                                }

                                if (createAction == InputAction.CursorDown)
                                {
                                    form.MoveDown(visibleFields);
                                    dirty = true;
                                    continue;
                                }

                                if (createAction is InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
                                {
                                    if (createAction == InputAction.CycleFocusPrev) form.MoveUp(visibleFields);
                                    else form.MoveDown(visibleFields);
                                    createError = string.Empty;
                                    dirty = true;
                                    continue;
                                }

                                if (key.Key == ConsoleKey.Spacebar)
                                {
                                    switch (form.SelectedField)
                                    {
                                        case ServerCreateForm.Field.OnlineMode: form.ToggleOnlineMode(); break;
                                        case ServerCreateForm.Field.Whitelist: form.ToggleWhitelist(); break;
                                        case ServerCreateForm.Field.RconEnabled: form.ToggleRconEnabled(); break;
                                        case ServerCreateForm.Field.Advanced:
                                            advanced = !advanced;
                                            if (!advanced) page = CreatePage.Basic;
                                            break;
                                        default: continue;
                                    }

                                    createError = string.Empty;
                                    dirty = true;
                                    continue;
                                }

                                if (createAction is InputAction.Confirm or InputAction.OpenCommand)
                                {
                                    if (form.SelectedField == ServerCreateForm.Field.Submit)
                                    {
                                        if (string.IsNullOrWhiteSpace(form.Name)) { createError = "Server name required"; dirty = true; continue; }
                                        if (!IsValidPort(form.ServerPort)) { createError = "Server port must be 1-65535"; dirty = true; continue; }
                                        if (form.MaxPlayers is < 1 or > 10_000) { createError = "Max players must be 1-10000"; dirty = true; continue; }
                                        if (form.ViewDistance is < 2 or > 32) { createError = "View distance must be 2-32"; dirty = true; continue; }
                                        if (string.IsNullOrWhiteSpace(form.LevelName)) { createError = "Level name required"; dirty = true; continue; }
                                        if (string.IsNullOrWhiteSpace(form.Difficulty)) { createError = "Difficulty required"; dirty = true; continue; }

                                        if (form.RconEnabled)
                                        {
                                            if (!IsValidPort(form.RconPort)) { createError = "RCON port must be 1-65535"; dirty = true; continue; }
                                            if (form.ServerPort == form.RconPort) { createError = "Server port and RCON port must differ"; dirty = true; continue; }
                                            if (string.IsNullOrWhiteSpace(form.RconPassword)) { createError = "RCON password required"; dirty = true; continue; }
                                            if (form.RconTimeoutSeconds is < 1 or > 120) { createError = "RCON timeout must be 1-120"; dirty = true; continue; }
                                        }

                                        createError = string.Empty;
                                        eulaCursorYes = true;
                                        mode = CreateMode.ConfirmEula;
                                        dirty = true;
                                        continue;
                                    }

                                    if (form.SelectedField == ServerCreateForm.Field.Version)
                                    {
                                        versionOriginal = form.Version;
                                        versionQuery = string.Empty;
                                        versionCursor = FindIndex(versions, form.Version);
                                        if (versionCursor < 0) versionCursor = 0;
                                        mode = CreateMode.SelectVersion;
                                        createError = string.Empty;
                                        dirty = true;
                                        continue;
                                    }

                                    if (form.SelectedField == ServerCreateForm.Field.Type)
                                    {
                                        typeOriginal = form.Type;
                                        typeCursor = Array.IndexOf(form.Types, form.Type);
                                        if (typeCursor < 0) typeCursor = 0;
                                        mode = CreateMode.SelectType;
                                        createError = string.Empty;
                                        dirty = true;
                                        continue;
                                    }

                                    if (form.IsTextEditable(form.SelectedField))
                                    {
                                        editOriginal = form.GetTextValue(form.SelectedField);
                                        editBuffer = editOriginal;
                                        mode = CreateMode.EditText;
                                        createError = string.Empty;
                                        dirty = true;
                                        continue;
                                    }

                                    switch (form.SelectedField)
                                    {
                                        case ServerCreateForm.Field.OnlineMode: form.ToggleOnlineMode(); break;
                                        case ServerCreateForm.Field.Whitelist: form.ToggleWhitelist(); break;
                                        case ServerCreateForm.Field.RconEnabled: form.ToggleRconEnabled(); break;
                                        case ServerCreateForm.Field.Advanced:
                                            advanced = !advanced;
                                            if (!advanced) page = CreatePage.Basic;
                                            break;
                                    }

                                    createError = string.Empty;
                                    dirty = true;
                                    continue;
                                }

                                continue;
                            }

                            if (mode == CreateMode.EditText)
                            {
                                if (createAction == InputAction.TextBackspace)
                                {
                                    if (editBuffer.Length > 0) editBuffer = editBuffer[..^1];
                                    dirty = true;
                                    continue;
                                }

                                if (createAction is InputAction.Confirm or InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
                                {
                                    switch (form.SelectedField)
                                    {
                                        case ServerCreateForm.Field.Name: form.SetName(editBuffer); break;
                                        case ServerCreateForm.Field.Directory: form.SetDirectory(editBuffer); break;
                                        case ServerCreateForm.Field.ServerPort:
                                            if (!TryParsePort(editBuffer, out var sp)) { createError = "Server port must be 1-65535"; dirty = true; continue; }
                                            form.ServerPort = sp;
                                            break;
                                        case ServerCreateForm.Field.MaxPlayers:
                                            if (!int.TryParse(editBuffer.Trim(), out var mp) || mp is < 1 or > 10_000) { createError = "Max players must be 1-10000"; dirty = true; continue; }
                                            form.MaxPlayers = mp;
                                            break;
                                        case ServerCreateForm.Field.MotD: form.SetMotD(editBuffer); break;
                                        case ServerCreateForm.Field.ViewDistance:
                                            if (!int.TryParse(editBuffer.Trim(), out var vd) || vd is < 2 or > 32) { createError = "View distance must be 2-32"; dirty = true; continue; }
                                            form.ViewDistance = vd;
                                            break;
                                        case ServerCreateForm.Field.LevelName: form.SetLevelName(editBuffer); break;
                                        case ServerCreateForm.Field.Difficulty: form.SetDifficulty(editBuffer); break;
                                        case ServerCreateForm.Field.RconPort:
                                            if (!TryParsePort(editBuffer, out var rp)) { createError = "RCON port must be 1-65535"; dirty = true; continue; }
                                            form.RconPort = rp;
                                            break;
                                        case ServerCreateForm.Field.RconPassword: form.SetRconPassword(editBuffer); break;
                                        case ServerCreateForm.Field.RconTimeoutSeconds:
                                            if (!int.TryParse(editBuffer.Trim(), out var rt) || rt is < 1 or > 120) { createError = "RCON timeout must be 1-120"; dirty = true; continue; }
                                            form.RconTimeoutSeconds = rt;
                                            break;
                                        case ServerCreateForm.Field.JvmMinMemory: form.SetJvmMinMemory(editBuffer); break;
                                        case ServerCreateForm.Field.JvmMaxMemory: form.SetJvmMaxMemory(editBuffer); break;
                                        case ServerCreateForm.Field.JvmAdditionalFlags: form.JvmAdditionalFlags = editBuffer; break;
                                    }

                                    editBuffer = string.Empty;
                                    mode = CreateMode.Normal;
                                    createError = string.Empty;

                                    if (createAction == InputAction.CycleFocusPrev) form.MoveUp(visibleFields);
                                    else if (createAction == InputAction.CycleFocusNext) form.MoveDown(visibleFields);

                                    dirty = true;
                                    continue;
                                }

                                if (!char.IsControl(key.KeyChar))
                                {
                                    editBuffer += key.KeyChar;
                                    dirty = true;
                                    continue;
                                }

                                continue;
                            }

                            if (mode == CreateMode.SelectVersion)
                            {
                                var filtered = FilterVersions(versions, versionQuery);
                                versionCursor = filtered.Count == 0 ? 0 : Math.Clamp(versionCursor, 0, filtered.Count - 1);

                                if (createAction == InputAction.CursorUp) { if (versionCursor > 0) versionCursor--; dirty = true; continue; }
                                if (createAction == InputAction.CursorDown) { if (filtered.Count > 0 && versionCursor < filtered.Count - 1) versionCursor++; dirty = true; continue; }
                                if (createAction == InputAction.TextBackspace) { if (versionQuery.Length > 0) versionQuery = versionQuery[..^1]; versionCursor = 0; dirty = true; continue; }

                                if (createAction is InputAction.Confirm or InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
                                {
                                    if (filtered.Count > 0)
                                        form.Version = filtered[versionCursor];

                                    versionQuery = string.Empty;
                                    versionCursor = 0;
                                    mode = CreateMode.Normal;
                                    createError = string.Empty;

                                    if (createAction == InputAction.CycleFocusPrev) form.MoveUp(visibleFields);
                                    else if (createAction == InputAction.CycleFocusNext) form.MoveDown(visibleFields);

                                    dirty = true;
                                    continue;
                                }

                                if (createAction == InputAction.OpenCommand)
                                    continue;

                                if (!char.IsControl(key.KeyChar))
                                {
                                    versionQuery += key.KeyChar;
                                    versionCursor = 0;
                                    dirty = true;
                                    continue;
                                }

                                continue;
                            }

                            if (mode == CreateMode.SelectType)
                            {
                                var types = form.Types;
                                typeCursor = Math.Clamp(typeCursor, 0, Math.Max(0, types.Length - 1));

                                if (createAction == InputAction.CursorUp) { if (typeCursor > 0) typeCursor--; dirty = true; continue; }
                                if (createAction == InputAction.CursorDown) { if (types.Length > 0 && typeCursor < types.Length - 1) typeCursor++; dirty = true; continue; }

                                if (createAction is InputAction.Confirm or InputAction.CycleFocusNext or InputAction.CycleFocusPrev)
                                {
                                    if (types.Length > 0)
                                    {
                                        form.Type = types[typeCursor];
                                        versions = versionsByType.TryGetValue(form.Type, out var tv)
                                            ? tv
                                            : Array.Empty<string>();

                                        versionQuery = string.Empty;
                                        versionCursor = FindIndex(versions, form.Version);
                                        if (versionCursor < 0) versionCursor = 0;
                                    }

                                    mode = CreateMode.Normal;
                                    createError = string.Empty;

                                    if (createAction == InputAction.CycleFocusPrev) form.MoveUp(visibleFields);
                                    else if (createAction == InputAction.CycleFocusNext) form.MoveDown(visibleFields);

                                    dirty = true;
                                    continue;
                                }

                                continue;
                            }

                            if (mode == CreateMode.ConfirmEula)
                            {
                                if (createAction is InputAction.TabLeft or InputAction.TabRight
                                    || createAction is InputAction.CursorUp or InputAction.CursorDown)
                                {
                                    eulaCursorYes = !eulaCursorYes;
                                    dirty = true;
                                    continue;
                                }

                                if (createAction == InputAction.Confirm)
                                {
                                    form.AcceptEula = eulaCursorYes;
                                    if (!form.AcceptEula)
                                    {
                                        createError = "You must accept the EULA to create";
                                        mode = CreateMode.Normal;
                                        dirty = true;
                                        continue;
                                    }

                                    createError = string.Empty;
                                    result = new CreateModalResult(form);
                                    return;
                                }

                                continue;
                            }
                        }
                    });

                if (result is not null)
                    return result;
            }
        }
        catch (OperationCanceledException)
        {
        }
        finally
        {
            Console.CursorVisible = false;
            AnsiConsole.Clear();
        }

        return new CreateModalResult(null);
    }

    private static void UpdateLayout(
        Layout layout,
        ServerCreateForm form,
        IReadOnlyList<string> versions,
        CreateMode mode,
        CreatePage page,
        bool advanced,
        string editBuffer,
        string _,
        ref string versionQuery,
        int versionCursor,
        int typeCursor,
        bool eulaCursorYes,
        string createError)
    {
        var visibleFields = GetCreateVisibleFields(page, advanced, form.RconEnabled);
        if (visibleFields.IndexOf(form.SelectedField) < 0)
            form.SelectedField = visibleFields.Count > 0 ? visibleFields[0] : ServerCreateForm.Field.Submit;

        var table = RenderCreateFormTable(
            form,
            page,
            advanced,
            mode,
            editBuffer,
            versionQuery,
            versionCursor,
            typeCursor,
            versions,
            eulaCursorYes);

        var pageTitle = advanced
            ? page switch
            {
                CreatePage.Basic => "Basic",
                CreatePage.Gameplay => "Gameplay",
                CreatePage.Network => "Network",
                CreatePage.Java => "Java",
                _ => "",
            }
            : "Basic";

        var help = mode switch
        {
            CreateMode.Normal => advanced
                ? $"[dim]{Markup.Escape(pageTitle)}  ↑↓/Tab:nav  ←→:page  A:advanced  Enter:activate  Space:toggle  Esc:cancel[/]"
                : "[dim]Basic  ↑↓/Tab:nav  A:advanced  Enter:activate  Space:toggle  Esc:cancel[/]",
            CreateMode.EditText =>
                "[dim]Type to edit  Enter:confirm  Esc:cancel  Tab:confirm+next  Backspace:delete[/]",
            CreateMode.SelectVersion =>
                "[dim]↑↓:select  Type:search  Enter:confirm  Esc:cancel  Tab:confirm+next  Backspace:delete[/]",
            CreateMode.SelectType => "[dim]↑↓:select  Enter:confirm  Esc:cancel  Tab:confirm+next[/]",
            CreateMode.ConfirmEula => "[dim]←→:choose  Enter:confirm  Esc:back[/]",
            _ => "[dim][/]"
        };

        var helpMarkup = string.IsNullOrWhiteSpace(createError)
            ? $"\n{help}"
            : $"[bold red]{Markup.Escape(createError)}[/]\n{help}";

        var formPanel = new Panel(table)
        {
            Expand = false,
            Border = BoxBorder.None,
        };

        layout["Form"].Update(new Align(formPanel, HorizontalAlignment.Center, VerticalAlignment.Middle));
        layout["Help"].Update(new Align(new Markup(helpMarkup), HorizontalAlignment.Center));
    }

    private static CreatePage NextCreatePage(CreatePage page) => page switch
    {
        CreatePage.Basic => CreatePage.Gameplay,
        CreatePage.Gameplay => CreatePage.Network,
        CreatePage.Network => CreatePage.Java,
        CreatePage.Java => CreatePage.Basic,
        _ => CreatePage.Basic,
    };

    private static CreatePage PrevCreatePage(CreatePage page) => page switch
    {
        CreatePage.Basic => CreatePage.Java,
        CreatePage.Gameplay => CreatePage.Basic,
        CreatePage.Network => CreatePage.Gameplay,
        CreatePage.Java => CreatePage.Network,
        _ => CreatePage.Basic,
    };

    private static IReadOnlyList<ServerCreateForm.Field> GetCreateVisibleFields(CreatePage page, bool advanced,
        bool rconEnabled)
    {
        var fields = new List<ServerCreateForm.Field>(16);

        if (page == CreatePage.Basic)
        {
            fields.Add(ServerCreateForm.Field.Name);
            fields.Add(ServerCreateForm.Field.Type);
            fields.Add(ServerCreateForm.Field.Version);
            fields.Add(ServerCreateForm.Field.ServerPort);
            fields.Add(ServerCreateForm.Field.MaxPlayers);
            fields.Add(ServerCreateForm.Field.Advanced);
        }
        else if (page == CreatePage.Gameplay)
        {
            fields.Add(ServerCreateForm.Field.MotD);
            fields.Add(ServerCreateForm.Field.ViewDistance);
            fields.Add(ServerCreateForm.Field.OnlineMode);
            fields.Add(ServerCreateForm.Field.Whitelist);
            fields.Add(ServerCreateForm.Field.LevelName);
            fields.Add(ServerCreateForm.Field.Difficulty);
        }
        else if (page == CreatePage.Network)
        {
            fields.Add(ServerCreateForm.Field.Directory);
            fields.Add(ServerCreateForm.Field.RconEnabled);
            if (rconEnabled)
            {
                fields.Add(ServerCreateForm.Field.RconPort);
                fields.Add(ServerCreateForm.Field.RconPassword);
                fields.Add(ServerCreateForm.Field.RconTimeoutSeconds);
            }
        }
        else if (page == CreatePage.Java)
        {
            fields.Add(ServerCreateForm.Field.JvmMinMemory);
            fields.Add(ServerCreateForm.Field.JvmMaxMemory);
            fields.Add(ServerCreateForm.Field.JvmAdditionalFlags);
        }

        if (!advanced)
        {
            return new List<ServerCreateForm.Field>
            {
                ServerCreateForm.Field.Name,
                ServerCreateForm.Field.Type,
                ServerCreateForm.Field.Version,
                ServerCreateForm.Field.ServerPort,
                ServerCreateForm.Field.MaxPlayers,
                ServerCreateForm.Field.Advanced,
                ServerCreateForm.Field.Submit,
            };
        }

        fields.Add(ServerCreateForm.Field.Submit);
        return fields;
    }

    private static Table RenderCreateFormTable(
        ServerCreateForm form,
        CreatePage page,
        bool advanced,
        CreateMode mode,
        string editBuffer,
        string versionQuery,
        int versionCursor,
        int typeCursor,
        IReadOnlyList<string> allVersions,
        bool eulaCursorYes)
    {
        const int FormPadding = 4;

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
                ServerCreateForm.Field.Advanced => advanced ? "[green]ON[/]" : "[red]OFF[/]",
                _ => string.Empty
            };
        }

        string PlainValue(ServerCreateForm.Field field)
        {
            if (mode == CreateMode.EditText && form.SelectedField == field)
                return editBuffer + "█";

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
                ServerCreateForm.Field.Advanced => advanced ? "ON" : "OFF",
                _ => string.Empty
            };
        }

        var muted = mode is CreateMode.SelectVersion or CreateMode.SelectType;

        var fieldLabels = new Dictionary<ServerCreateForm.Field, string>
        {
            [ServerCreateForm.Field.Name] = "Name",
            [ServerCreateForm.Field.Type] = "Type",
            [ServerCreateForm.Field.Version] = "Version",
            [ServerCreateForm.Field.ServerPort] = "Server Port",
            [ServerCreateForm.Field.MaxPlayers] = "Max Players",
            [ServerCreateForm.Field.Advanced] = "Advanced",

            [ServerCreateForm.Field.MotD] = "MotD",
            [ServerCreateForm.Field.ViewDistance] = "View Dist",
            [ServerCreateForm.Field.OnlineMode] = "Online",
            [ServerCreateForm.Field.Whitelist] = "Whitelist",
            [ServerCreateForm.Field.LevelName] = "Level",
            [ServerCreateForm.Field.Difficulty] = "Difficulty",

            [ServerCreateForm.Field.Directory] = "Directory",
            [ServerCreateForm.Field.RconEnabled] = "RCON",
            [ServerCreateForm.Field.RconPort] = "RCON Port",
            [ServerCreateForm.Field.RconPassword] = "RCON Pass",
            [ServerCreateForm.Field.RconTimeoutSeconds] = "RCON T/O",

            [ServerCreateForm.Field.JvmMinMemory] = "Xms",
            [ServerCreateForm.Field.JvmMaxMemory] = "Xmx",
            [ServerCreateForm.Field.JvmAdditionalFlags] = "JVM Flags",
        };

        var navFields = GetCreateVisibleFields(page, advanced, form.RconEnabled);
        var fields = new List<(string Label, string Value, ServerCreateForm.Field Field)>(navFields.Count);
        for (var i = 0; i < navFields.Count; i++)
        {
            var f = navFields[i];
            if (f == ServerCreateForm.Field.Submit) continue;
            fields.Add((fieldLabels[f], Value(f), f));
        }

        var availableW = Math.Max(1, Console.WindowWidth - FormPadding);
        var seamTextW = 0;
        var valueTextW = 0;

        foreach (var (label, _, fieldEnum) in fields)
        {
            var prefix = form.SelectedField == fieldEnum && mode is CreateMode.Normal or CreateMode.EditText
                ? "→ "
                : "  ";
            var left = prefix + label;
            if (left.Length > seamTextW) seamTextW = left.Length;

            var pv = PlainValue(fieldEnum);
            if (pv.Length > valueTextW) valueTextW = pv.Length;
        }

        var buttonText = "[ Create Server ]";
        if (buttonText.Length > valueTextW) valueTextW = buttonText.Length;

        if (mode == CreateMode.ConfirmEula)
        {
            valueTextW = Math.Max(valueTextW, "Accept Minecraft EULA?".Length);
            valueTextW = Math.Max(valueTextW, "https://aka.ms/MinecraftEULA".Length);
            valueTextW = Math.Max(valueTextW, "  YES     NO  ".Length);
        }

        if (mode == CreateMode.SelectVersion)
        {
            var filtered = FilterVersions(allVersions, versionQuery);
            const int pageSize = 8;
            var cur = filtered.Count == 0 ? 0 : Math.Clamp(versionCursor, 0, filtered.Count - 1);
            var start = Math.Max(0, cur - (pageSize / 2));
            var end = Math.Min(filtered.Count, start + pageSize);
            if (end - start < pageSize && start > 0) start = Math.Max(0, end - pageSize);

            valueTextW = Math.Max(valueTextW, "...".Length);

            for (var i = start; i < end; i++)
            {
                var s = (i == cur ? "→ " : "  ") + filtered[i];
                if (s.Length > valueTextW) valueTextW = s.Length;
            }

            var search = $"Search: {versionQuery}█";
            if (search.Length > valueTextW) valueTextW = search.Length;
        }
        else if (mode == CreateMode.SelectType)
        {
            var types = form.Types;
            for (var i = 0; i < types.Length; i++)
            {
                var s = (i == Math.Clamp(typeCursor, 0, Math.Max(0, types.Length - 1)) ? "→ " : "  ") + types[i];
                if (s.Length > valueTextW) valueTextW = s.Length;
            }
        }

        var desiredW = Math.Max(17, (2 * Math.Max(seamTextW, valueTextW)) + 1);
        var totalW = Math.Min(availableW, desiredW);
        if (totalW % 2 == 0)
        {
            totalW = totalW < availableW ? totalW + 1 : totalW - 1;
        }

        var sideW = Math.Max(1, (totalW - 1) / 2);
        var leftW = sideW;
        var rightW = totalW - 1 - leftW;

        var formTable = new Table()
            .HideHeaders()
            .NoBorder()
            .Collapse()
            .AddColumn(new TableColumn(string.Empty).NoWrap().RightAligned())
            .AddColumn(new TableColumn(string.Empty).NoWrap().Centered())
            .AddColumn(new TableColumn(string.Empty).NoWrap().LeftAligned());

        formTable.Columns[0].Width(leftW);
        formTable.Columns[1].Width(1);
        formTable.Columns[2].Width(rightW);

        foreach (var (label, value, fieldEnum) in fields)
        {
            var isSelected = form.SelectedField == fieldEnum && mode is CreateMode.Normal or CreateMode.EditText;
            var labelStyle = isSelected ? "bold yellow reverse" : (muted ? "dim" : "white");
            var valueStyle = isSelected ? "bold cyan reverse" : (muted ? "dim" : "cyan");

            var prefix = isSelected ? "→ " : "  ";
            formTable.AddRow(
                new Markup($"[{labelStyle}]{Markup.Escape(prefix + label)}[/]"),
                new Markup("[dim]:[/]"),
                new Markup($"[{valueStyle}]{value}[/]"));
        }

        var content = new Table()
            .HideHeaders()
            .NoBorder()
            .Collapse()
            .AddColumn(new TableColumn(string.Empty).NoWrap().Centered());

        content.AddRow(new Align(formTable, HorizontalAlignment.Center));

        if (mode == CreateMode.SelectVersion)
        {
            content.AddRow(new Align(new Markup($"[dim]{new string('─', totalW)}[/]"), HorizontalAlignment.Center));

            var list = new Table()
                .HideHeaders()
                .NoBorder()
                .Collapse()
                .AddColumn(new TableColumn(string.Empty).NoWrap().Centered().Width(totalW));

            var filtered = FilterVersions(allVersions, versionQuery);
            const int pageSize = 8;
            var cur = filtered.Count == 0 ? 0 : Math.Clamp(versionCursor, 0, filtered.Count - 1);
            var start = Math.Max(0, cur - (pageSize / 2));
            var end = Math.Min(filtered.Count, start + pageSize);
            if (end - start < pageSize && start > 0) start = Math.Max(0, end - pageSize);

            if (start > 0)
                list.AddRow(new Markup("[dim]...[/]"));

            for (var i = start; i < end; i++)
            {
                var v = Markup.Escape(filtered[i]);
                var sel = i == cur;
                var style = sel ? "bold cyan reverse" : "white";
                var pfx = sel ? "→ " : "  ";
                list.AddRow(new Markup($"[{style}]{pfx}{v}[/]"));
            }

            if (end < filtered.Count)
                list.AddRow(new Markup("[dim]...[/]"));

            var q = Markup.Escape(versionQuery);
            list.AddRow(new Markup($"[dim]Search:[/] [bold]{q}[/][dim]█[/]"));

            content.AddRow(new Align(list, HorizontalAlignment.Center));
        }
        else if (mode == CreateMode.SelectType)
        {
            content.AddRow(new Align(new Markup($"[dim]{new string('─', totalW)}[/]"), HorizontalAlignment.Center));

            var list = new Table()
                .HideHeaders()
                .NoBorder()
                .Collapse()
                .AddColumn(new TableColumn(string.Empty).NoWrap().Centered().Width(totalW));

            var types = form.Types;
            var cur = Math.Clamp(typeCursor, 0, Math.Max(0, types.Length - 1));
            for (var i = 0; i < types.Length; i++)
            {
                var t = Markup.Escape(types[i].ToString());
                var sel = i == cur;
                var style = sel ? "bold cyan reverse" : "white";
                var pfx = sel ? "→ " : "  ";
                list.AddRow(new Markup($"[{style}]{pfx}{t}[/]"));
            }

            content.AddRow(new Align(list, HorizontalAlignment.Center));
        }

        if (mode is CreateMode.Normal or CreateMode.EditText)
        {
            var btnSelected = form.SelectedField == ServerCreateForm.Field.Submit && mode == CreateMode.Normal;
            var btnStyle = btnSelected ? "bold green reverse" : "green";
            content.AddRow(new Markup(string.Empty));
            content.AddRow(new Align(
                new Markup($"[{btnStyle}]{Markup.Escape("[ Create Server ]")}[/]"),
                HorizontalAlignment.Center));
        }

        if (mode == CreateMode.ConfirmEula)
        {
            var hr = new string('─', totalW);
            content.AddRow(new Markup(string.Empty));
            content.AddRow(new Align(new Markup($"[dim]{hr}[/]"), HorizontalAlignment.Center));

            var yesStyle = eulaCursorYes ? "bold green reverse" : "green";
            var noStyle = !eulaCursorYes ? "bold red reverse" : "red";
            var prompt = new Table()
                .HideHeaders()
                .NoBorder()
                .Collapse()
                .AddColumn(new TableColumn(string.Empty).NoWrap().Centered().Width(totalW));
            prompt.AddRow(new Markup("[bold]Accept Minecraft EULA?[/]"));
            prompt.AddRow(new Markup("[dim]https://aka.ms/MinecraftEULA[/]"));
            prompt.AddRow(new Markup(string.Empty));
            prompt.AddRow(new Markup($"[{yesStyle}]  YES  [/]   [{noStyle}]  NO  [/]"));

            content.AddRow(new Align(prompt, HorizontalAlignment.Center));
        }

        return content;
    }
}
