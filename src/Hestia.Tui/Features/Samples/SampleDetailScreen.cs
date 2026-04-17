using Hestia.Tui.Input;
using Hestia.Tui.Navigation;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Samples;

/// <summary>
/// Demonstrates receiving data from a parent screen and popping back.
/// </summary>
public sealed class SampleDetailScreen(string item, INavigator navigator) : ScreenBase
{
    private Layout? _layout;

    public override IRenderable Render()
    {
        _layout ??= new Layout("Root").SplitRows(
            new Layout("Content"),
            new Layout("Footer").Size(1)
        );

        _layout["Content"].Update(
            new Panel(
                new Markup($"[bold]Item:[/] [cyan]{item}[/]\n\n[dim]This is the detail view for the selected item.[/]")
            )
                .Header($"[bold]Detail: {item}[/]")
                .BorderColor(Color.Cyan1)
                .Expand()
        );

        _layout["Footer"].Update(
            new Markup("[dim] Press [b]Esc[/] or [b]Q[/] to go back[/]")
        );

        return _layout;
    }

    public override void OnInput(InputAction action)
    {
        if (action is InputAction.Back or InputAction.Quit)
            navigator.Pop();
    }
}
