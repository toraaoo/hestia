using Hestia.Core.Minecraft;
using Hestia.Core.Minecraft.Models;
using Hestia.Tui.Input;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard.Tabs;

public interface ITab
{
    string Title { get; }
    IRenderable Render(Server? server);
    Task OnServerChangedAsync(Server? server, CancellationToken ct) => Task.CompletedTask;
    void OnInput(InputAction action) { }
    bool OnRawKey(ConsoleKeyInfo key) => false;
}
