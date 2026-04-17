using Hestia.Tui.Input;
using Hestia.Tui.Modals;

namespace Hestia.Tui.Navigation;

/// <summary>
/// Singleton navigation service injected into screens.
/// ScreenStack activates/deactivates the current host around each OnInput call.
/// </summary>
public sealed class Navigator : INavigator
{
    private ScreenHost? _host;

    internal void Activate(ScreenHost host) => _host = host;
    internal void Deactivate() => _host = null;

    public void Push(IScreen screen) => _host?.Push(screen);
    public void Pop() => _host?.Pop();
    public void Quit() => _host?.Quit();
    public void ShowModal<TResult>(IModal<TResult> modal, Action<TResult> onResult)
        => _host?.ShowModal(modal, onResult);
}

/// <summary>
/// Captures navigation intent raised during a single OnInput call.
/// ScreenStack reads state after the live loop iteration ends.
/// </summary>
internal sealed class ScreenHost(KeyMap keyMap)
{
    public IScreen? PendingPush { get; private set; }
    public bool PendingPop { get; private set; }
    public Func<CancellationToken, Task>? PendingModal { get; private set; }
    public bool WantsQuit { get; private set; }

    public bool HasPendingNavigation =>
        PendingPush != null || PendingPop || PendingModal != null || WantsQuit;

    public void Push(IScreen screen)
    {
        if (!HasPendingNavigation) PendingPush = screen;
    }

    public void Pop()
    {
        if (!HasPendingNavigation) PendingPop = true;
    }

    public void ShowModal<TResult>(IModal<TResult> modal, Action<TResult> onResult)
    {
        if (!HasPendingNavigation)
            PendingModal = async ct =>
            {
                var result = await modal.ShowAsync(keyMap, ct);
                onResult(result);
            };
    }

    public void Quit()
    {
        if (!HasPendingNavigation) WantsQuit = true;
    }
}
