using Hestia.Tui.Features.Samples;
using Hestia.Tui.Input;
using Hestia.Tui.Navigation;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Dashboard;

public sealed class DashboardScreen : ScreenBase
{
    private Layout? _layout;

    public override IRenderable Render()
    {
        if (_layout != null) return _layout;

        _layout = new Layout("Root")
            .SplitRows(
                new Layout("Main"),
                new Layout("Footer").Size(1)
            );

        _layout["Main"].SplitColumns(
            new Layout("Left").Ratio(25),
            new Layout("Content").Ratio(75)
        );

        _layout["Footer"].Update(
            new Markup("[dim]Press [b]Enter[/] sample · [b]Q[/] quit[/]")
        );

        return _layout;
    }

    public override void OnInput(InputAction action)
    {
        switch (action)
        {
            case InputAction.Confirm:
                ScreenContext.Host.Push(new SampleScreen());
                break;
            case InputAction.Back:
            case InputAction.Quit:
                ScreenContext.Host.Quit();
                break;
        }
    }
}
