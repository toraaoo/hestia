using Hestia.Tui.Modals;

namespace Hestia.Tui.Navigation;

public interface IScreenHost
{
    /// <summary>Pushes a new screen on top of the current one.</summary>
    void Push(IScreen screen);

    /// <summary>Removes the current screen, returning to the previous one.</summary>
    void Pop();

    /// <summary>
    /// Pauses the live loop, shows a full-screen modal, then invokes
    /// <paramref name="onResult"/> with the result before resuming.
    /// </summary>
    void ShowModal<TResult>(IModal<TResult> modal, Action<TResult> onResult);

    /// <summary>Signals the app to exit.</summary>
    void Quit();
}
