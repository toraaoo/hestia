using Hestia.Core.Minecraft.Models;
using Hestia.Tui.Input;
using Hestia.Tui.Navigation;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard.Tabs;

public abstract class Tab : IView
{
    public abstract string Title { get; }

    protected Server? Server { get; private set; }

    public abstract IRenderable Render();

    public virtual Task OnServerChangedAsync(Server? server, CancellationToken ct)
    {
        Server = server;
        return Task.CompletedTask;
    }

    public virtual void OnInput(InputAction action) { }
    public virtual bool OnRawKey(ConsoleKeyInfo key) => false;
}