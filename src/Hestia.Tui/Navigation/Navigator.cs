using Hestia.Tui.Modals;

namespace Hestia.Tui.Navigation;

/// <summary>
/// Captures navigation intents raised by a screen during <see cref="IScreen.OnInput"/>.
/// <see cref="ScreenStack"/> reads these after the live loop exits each iteration.
/// </summary>
internal sealed class ScreenHost : IScreenHost
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
                var result = await modal.ShowAsync(ct);
                onResult(result);
            };
    }

    public void Quit()
    {
        if (!HasPendingNavigation) WantsQuit = true;
    }
}
