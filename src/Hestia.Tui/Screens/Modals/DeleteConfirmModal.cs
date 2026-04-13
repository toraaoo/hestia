using Spectre.Console;
using static Hestia.Tui.Utilities.TerminalUtils;

namespace Hestia.Tui.Screens.Modals;

internal static class DeleteConfirmModal
{
    public static async Task<DeleteModalResult> RunAsync(string serverName, CancellationToken ct)
    {
        Console.CursorVisible = false;

        try
        {
            var table = new Table()
                .HideHeaders()
                .NoBorder()
                .AddColumn(new TableColumn(string.Empty).Centered());

            table.AddRow(new Markup("[bold red]Delete server?[/]"));
            table.AddRow(new Markup($"[dim]{Markup.Escape(serverName)}[/]"));

            var layout = new Layout()
                .SplitRows(
                    new Layout("Form"),
                    new Layout("Help").Size(2));

            layout["Form"].Update(new Align(table, HorizontalAlignment.Center, VerticalAlignment.Middle));
            layout["Help"].Update(new Align(new Markup("[dim]Y:confirm  N:cancel[/]"), HorizontalAlignment.Center));

            while (!ct.IsCancellationRequested && TooSmall())
            {
                AnsiConsole.Clear();
                Console.WriteLine($"Terminal too small. Resize to at least {MinWidth}×{MinHeight}.");
                await Task.Delay(300, ct);
            }

            if (ct.IsCancellationRequested)
                return new DeleteModalResult(false);

            DeleteModalResult? result = null;
            await AnsiConsole.Live(layout)
                .AutoClear(false)
                .Overflow(VerticalOverflow.Ellipsis)
                .Cropping(VerticalOverflowCropping.Bottom)
                .StartAsync(async ctx =>
                {
                    ctx.Refresh();

                    while (!ct.IsCancellationRequested && result is null)
                    {
                        if (TooSmall())
                            return;

                        if (!Console.KeyAvailable)
                        {
                            await Task.Delay(50, ct);
                            continue;
                        }

                        var key = Console.ReadKey(true);
                        if (key.KeyChar == 'y' || key.KeyChar == 'Y')
                        {
                            result = new DeleteModalResult(true);
                            return;
                        }

                        if (key.KeyChar == 'n' || key.KeyChar == 'N' || key.Key == ConsoleKey.Escape)
                        {
                            result = new DeleteModalResult(false);
                            return;
                        }
                    }
                });

            return result ?? new DeleteModalResult(false);
            }
        finally
        {
            Console.CursorVisible = false;
            AnsiConsole.Clear();
        }
    }
}
