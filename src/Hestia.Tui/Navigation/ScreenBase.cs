using Hestia.Tui.Input;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Navigation;

public abstract class ScreenBase : IScreen
{
    public abstract IRenderable Render();
    public virtual void OnInput(InputAction action) { }
    public virtual Task LoadAsync(CancellationToken ct) => Task.CompletedTask;
}
