using Hestia.Tui.Modals;

namespace Hestia.Tui.Navigation;

public interface INavigator
{
    void Push(IScreen screen);
    void Pop();
    void ShowModal<TResult>(IModal<TResult> modal, Action<TResult> onResult);
    void Quit();
}
