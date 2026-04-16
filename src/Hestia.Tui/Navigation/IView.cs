using Hestia.Tui.Input;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Navigation;

public interface IView
{
    IRenderable Render();
    void OnInput(InputAction action);
    Task LoadAsync(CancellationToken ct) => Task.CompletedTask;
}
