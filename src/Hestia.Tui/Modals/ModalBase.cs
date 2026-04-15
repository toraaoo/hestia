using Hestia.Tui.Input;
using Hestia.Tui.Navigation;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Modals;

public abstract class ModalBase<TResult> : IModal<TResult>, IView
{
    private const int MinWidth = 60;
    private const int MinHeight = 10;

    private TaskCompletionSource<TResult>? _tcs;

    public abstract IRenderable Render();

    public virtual void OnInput(InputAction action) { }

    protected void Complete(TResult result) => _tcs?.TrySetResult(result);

    public async Task<TResult> ShowAsync(CancellationToken ct)
    {
        await WaitForTerminalSizeAsync(ct);

        _tcs = new TaskCompletionSource<TResult>(TaskCreationOptions.RunContinuationsAsynchronously);
        ct.Register(() => _tcs.TrySetCanceled());

        AnsiConsole.Clear();

        await AnsiConsole.Live(Render())
            .StartAsync(async ctx =>
            {
                while (!_tcs.Task.IsCompleted && !ct.IsCancellationRequested)
                {
                    ctx.UpdateTarget(Render());
                    ctx.Refresh();

                    if (Console.KeyAvailable)
                    {
                        var key = Console.ReadKey(true);
                        if (ScreenContext.KeyMap.Resolve(key) is { } action)
                            OnInput(action);
                    }

                    try
                    {
                        await Task.Delay(50, ct).ConfigureAwait(false);
                    }
                    catch (OperationCanceledException)
                    {
                        break;
                    }
                }
            });

        return await _tcs.Task;
    }

    private static async Task WaitForTerminalSizeAsync(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested &&
               (Console.WindowWidth < MinWidth || Console.WindowHeight < MinHeight))
        {
            AnsiConsole.Clear();
            AnsiConsole.MarkupLine($"[yellow]Terminal too small. Minimum {MinWidth}×{MinHeight}.[/]");
            await Task.Delay(500, ct).ConfigureAwait(false);
        }
    }
}
