using Spectre.Console;
using Spectre.Console.Rendering;
using static Hestia.Tui.Utilities.TerminalUtils;

namespace Hestia.Tui.Screens.Modals;

internal static class ProgressModal
{
    public static async Task RunAsync(ProgressState state, CancellationToken ct)
    {
        while (!ct.IsCancellationRequested && TooSmall())
        {
            AnsiConsole.Clear();
            Console.WriteLine($"Terminal too small. Resize to at least {MinWidth}×{MinHeight}.");
            await Task.Delay(300, ct);
        }

        if (ct.IsCancellationRequested)
            return;

        var last = Snapshot(state);
        var dirty = true;

        await AnsiConsole.Live(BuildLayout(state))
            .AutoClear(false)
            .Overflow(VerticalOverflow.Ellipsis)
            .Cropping(VerticalOverflowCropping.Bottom)
            .StartAsync(async ctx =>
            {
                while (!ct.IsCancellationRequested)
                {
                    if (state.IsComplete || state.HasError)
                        return;

                    if (TooSmall())
                        return;

                    var cur = Snapshot(state);
                    if (!dirty && cur.Equals(last))
                    {
                        await Task.Delay(100, ct);
                        continue;
                    }

                    ctx.UpdateTarget(BuildLayout(state));
                    ctx.Refresh();
                    dirty = false;
                    last = cur;

                    await Task.Delay(100, ct);
                }
            });
    }

    private static ProgressSnapshot Snapshot(ProgressState state) =>
        new(
            state.ServerName,
            state.Version,
            state.Type,
            Math.Round(state.Progress, 4),
            state.StatusMsg,
            state.IsComplete,
            state.HasError);

    private readonly record struct ProgressSnapshot(
        string ServerName,
        string Version,
        string Type,
        double Progress,
        string StatusMsg,
        bool IsComplete,
        bool HasError);

    private static IRenderable BuildLayout(ProgressState state)
    {
        var progress = Math.Clamp(state.Progress, 0.0, 1.0);
        var pct = (int)(progress * 100);
        var barFilled = (int)(progress * 30);
        var bar = new string('█', barFilled) + new string('░', 30 - barFilled);

        var grid = new Grid()
            .AddColumn(new GridColumn().Width(12).NoWrap())
            .AddColumn(new GridColumn().NoWrap());

        grid.AddRow(new Markup("[dim]Name[/]"), new Markup(Markup.Escape(state.ServerName)));
        grid.AddRow(new Markup("[dim]Version[/]"), new Markup(Markup.Escape(state.Version)));
        grid.AddRow(new Markup("[dim]Type[/]"), new Markup(Markup.Escape(state.Type)));
        grid.AddEmptyRow();
        grid.AddRow(new Markup("[dim]Status[/]"), new Markup(Markup.Escape(state.StatusMsg)));
        grid.AddRow(new Markup("[dim]Progress[/]"), new Markup($"[cyan]{bar}[/] [bold]{pct}%[/]"));

        var panel = new Panel(grid)
        {
            Header = new PanelHeader("[bold]Working[/]"),
            Border = BoxBorder.Rounded,
            Padding = new Padding(2, 1),
        };

        var layout = new Layout()
            .SplitRows(
                new Layout("content"),
                new Layout("help").Size(2));
        layout["content"].Update(new Align(panel, HorizontalAlignment.Center, VerticalAlignment.Middle));
        layout["help"].Update(new Align(new Markup("[dim]Please wait...[/]"), HorizontalAlignment.Center));

        return layout;
    }
}
