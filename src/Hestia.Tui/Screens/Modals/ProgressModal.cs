using Spectre.Console;
using Spectre.Console.Rendering;
using static Hestia.Tui.Utilities.TerminalUtils;

namespace Hestia.Tui.Screens.Modals;

internal static class ProgressModal
{
    public static async Task RunAsync(ProgressState state, CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            if (!TooSmall())
            {
                AnsiConsole.Clear();
                try
                {
                    AnsiConsole.Write(BuildLayout(state));
                }
                catch
                {
                }
            }
            else
            {
                AnsiConsole.Clear();
                Console.WriteLine($"Terminal too small. Resize to at least {MinWidth}×{MinHeight}.");
            }

            if (state.IsComplete || state.HasError)
                return;

            await Task.Delay(100, ct);
        }
    }

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
