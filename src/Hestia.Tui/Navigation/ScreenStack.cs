using Hestia.Tui.Input;
using Spectre.Console;

namespace Hestia.Tui.Navigation;

/// <summary>
/// Drives the screen lifecycle via a single persistent Spectre live loop.
/// Push/pop navigate without restarting the loop.
/// Only modals exit the loop (full-screen takeover), after which it restarts clean.
/// </summary>
public sealed class ScreenStack
{
    private readonly KeyMap _keyMap;

    public ScreenStack(KeyMap keyMap) => _keyMap = keyMap;

    public async Task RunAsync(IScreen initialScreen, CancellationToken ct)
    {
        var stack = new Stack<IScreen>();
        stack.Push(initialScreen);
        await initialScreen.LoadAsync(ct);
        var quit = false;

        while (stack.Count > 0 && !quit && !ct.IsCancellationRequested)
        {
            ScreenHost? modalHost = null;

            AnsiConsole.Clear();

            await AnsiConsole.Live(stack.Peek().Render())
                .StartAsync(async ctx =>
                {
                    // Outer: runs each screen in the stack without restarting the live loop.
                    while (stack.Count > 0 && modalHost == null && !quit && !ct.IsCancellationRequested)
                    {
                        var screen = stack.Peek();
                        var host = new ScreenHost();

                        // Inner: runs until this screen signals a navigation intent.
                        while (!host.HasPendingNavigation && !ct.IsCancellationRequested)
                        {
                            ctx.UpdateTarget(screen.Render());
                            ctx.Refresh();

                            if (Console.KeyAvailable)
                            {
                                var key = Console.ReadKey(true);
                                if (_keyMap.Resolve(key) is { } action)
                                {
                                    ScreenContext.Set(host, _keyMap);
                                    screen.OnInput(action);
                                }
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

                        if (host.WantsQuit)
                        {
                            quit = true;
                            return;
                        }

                        // Modal needs full-screen takeover — must exit live loop.
                        if (host.PendingModal != null)
                        {
                            modalHost = host;
                            return;
                        }

                        if (host.PendingPush != null)
                        {
                            stack.Push(host.PendingPush);
                            await host.PendingPush.LoadAsync(ct);
                        }
                        else if (host.PendingPop && stack.Count > 0)
                            stack.Pop();
                    }
                });

            if (modalHost?.PendingModal == null || quit) continue;

            // Re-set context: AsyncLocal values from inside StartAsync don't flow back out.
            ScreenContext.Set(modalHost, _keyMap);

            // Run modal outside live loop; its callback may set further push/pop on same host.
            await modalHost.PendingModal(ct);

            if (modalHost.PendingPush != null)
            {
                stack.Push(modalHost.PendingPush);
                await modalHost.PendingPush.LoadAsync(ct);
            }
            else if (modalHost.PendingPop && stack.Count > 0)
                stack.Pop();
        }
    }
}