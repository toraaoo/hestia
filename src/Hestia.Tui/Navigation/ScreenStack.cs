using Hestia.Tui.Input;
using Spectre.Console;

namespace Hestia.Tui.Navigation;

public sealed class ScreenStack(Navigator navigator, KeyMap keyMap)
{
    public async Task RunAsync(IScreen initialScreen, CancellationToken ct)
    {
        var stack = new Stack<IScreen>();
        stack.Push(initialScreen);
        await initialScreen.LoadAsync(ct);

        while (stack.Count > 0 && !ct.IsCancellationRequested)
        {
            AnsiConsole.Clear();
            var screen = stack.Peek();
            var host = new ScreenHost(keyMap);
            navigator.Activate(host);

            await AnsiConsole.Live(screen.Render())
                .StartAsync(async ctx =>
                {
                    while (!host.HasPendingNavigation && !ct.IsCancellationRequested)
                    {
                        ctx.UpdateTarget(screen.Render());
                        ctx.Refresh();

                        if (Console.KeyAvailable)
                        {
                            var key = Console.ReadKey(true);
                            if (keyMap.Resolve(key) is { } action)
                                screen.OnInput(action);
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

            navigator.Deactivate();

            if (host.WantsQuit) break;

            if (host.PendingModal != null)
            {
                await host.PendingModal(ct);
                if (host.PendingPush != null)
                {
                    stack.Push(host.PendingPush);
                    await host.PendingPush.LoadAsync(ct);
                }
                else if (host.PendingPop && stack.Count > 0)
                    stack.Pop();
            }
            else if (host.PendingPush != null)
            {
                stack.Push(host.PendingPush);
                await host.PendingPush.LoadAsync(ct);
            }
            else if (host.PendingPop && stack.Count > 0)
                stack.Pop();
        }
    }
}