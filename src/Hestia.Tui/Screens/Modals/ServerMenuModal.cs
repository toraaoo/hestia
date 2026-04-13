using Hestia.Core.Server;
using Hestia.Tui.Input;
using Spectre.Console;
using Spectre.Console.Rendering;
using static Hestia.Tui.Utilities.TerminalUtils;

namespace Hestia.Tui.Screens.Modals;

internal static class ServerMenuModal
{
    public static async Task<ServerMenuModalResult> RunAsync(
        MinecraftServer server,
        KeyMap keyMap,
        CancellationToken ct)
    {
        (string Label, InputAction Action)[] items =
        [
            ("Start", InputAction.ServerStart),
            ("Stop", InputAction.ServerStop),
            ("Restart", InputAction.ServerRestart),
            ("Delete", InputAction.ServerDelete),
        ];

        var cursor = 0;

        while (!ct.IsCancellationRequested && TooSmall())
        {
            AnsiConsole.Clear();
            Console.WriteLine($"Terminal too small. Resize to at least {MinWidth}×{MinHeight}.");
            await Task.Delay(300, ct);
        }

        if (ct.IsCancellationRequested)
            return new ServerMenuModalResult(null);

        ServerMenuModalResult? result = null;
        var dirty = true;

        await AnsiConsole.Live(BuildLayout(server, items, cursor))
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
                        ctx.UpdateTarget(BuildLayout(server, items, cursor));
                        ctx.Refresh();
                        dirty = false;
                    }

                    if (!Console.KeyAvailable)
                    {
                        await Task.Delay(50, ct);
                        continue;
                    }

                    var key = Console.ReadKey(true);
                    var action = keyMap.Translate(key);

                    if (action == InputAction.Escape)
                    {
                        result = new ServerMenuModalResult(null);
                        return;
                    }

                    if (action == InputAction.CursorUp && cursor > 0)
                    {
                        cursor--;
                        dirty = true;
                        continue;
                    }

                    if (action == InputAction.CursorDown && cursor < items.Length - 1)
                    {
                        cursor++;
                        dirty = true;
                        continue;
                    }

                    if (action == InputAction.Confirm)
                    {
                        result = new ServerMenuModalResult(items[cursor].Action);
                        return;
                    }
                }
            });

        return result ?? new ServerMenuModalResult(null);
    }

    private static IRenderable BuildLayout(
        MinecraftServer server,
        (string Label, InputAction Action)[] items,
        int cursor)
    {
        var stateColor = server.State switch
        {
            ServerState.Running => "green",
            ServerState.Starting => "yellow",
            ServerState.Stopping => "yellow",
            ServerState.Crashed => "red",
            _ => "dim",
        };

        var summary = new Grid()
            .AddColumn(new GridColumn().Width(10).NoWrap())
            .AddColumn(new GridColumn().NoWrap());

        summary.AddRow(new Markup("[dim]Name[/]"), new Markup(Markup.Escape(server.Name)));
        summary.AddRow(new Markup("[dim]Type[/]"), new Markup(Markup.Escape(server.Type.ToString())));
        summary.AddRow(new Markup("[dim]Version[/]"), new Markup(Markup.Escape(server.MinecraftVersion)));
        summary.AddRow(new Markup("[dim]State[/]"), new Markup($"[{stateColor}]{server.State}[/]"));
        summary.AddRow(new Markup("[dim]Port[/]"), new Markup(server.Options.Port.ToString()));
        summary.AddRow(
            new Markup("[dim]Memory[/]"),
            new Markup($"{server.JvmOptions.MinMemory} / {server.JvmOptions.MaxMemory}")
        );

        var actionTable = new Table()
            .HideHeaders()
            .NoBorder()
            .AddColumn(new TableColumn(string.Empty).NoWrap().Centered());

        foreach (var (i, item) in items.Select((x, i) => (i, x)))
        {
            var sel = i == cursor;
            var pfx = sel ? "→ " : "  ";
            var style = sel ? "bold cyan" : "white";
            actionTable.AddRow(new Markup($"[{style}]{pfx}{item.Label}[/]"));
        }

        var content = new Table()
            .HideHeaders()
            .NoBorder()
            .AddColumn(new TableColumn(string.Empty).Centered());

        content.AddRow(new Align(summary, HorizontalAlignment.Center));
        content.AddRow(new Align(new Markup("[dim]─────────────────────────────────[/]"), HorizontalAlignment.Center));
        content.AddRow(new Align(actionTable, HorizontalAlignment.Center));

        var panel = new Panel(content)
        {
            Expand = false,
            Border = BoxBorder.None,
        };

        var layout = new Layout()
            .SplitRows(
                new Layout("content"),
                new Layout("help").Size(2));

        layout["content"].Update(new Align(panel, HorizontalAlignment.Center, VerticalAlignment.Middle));
        layout["help"].Update(new Align(
            new Markup("[dim]↑↓:select  Enter:confirm  Esc:back[/]"),
            HorizontalAlignment.Center));

        return layout;
    }
}
