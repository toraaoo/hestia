using Hestia.Tui.Input;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard;

public interface IPanel
{
    void OnInput(InputAction action);
    bool OnRawKey(ConsoleKeyInfo key) => false;
    IRenderable Render(bool focused);
}
