using Hestia.Tui.Input;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Modals;

/// <summary>
/// Full-screen confirmation dialog. Returns true if the user confirmed.
/// Demonstrates how to implement <see cref="ModalBase{TResult}"/>.
/// </summary>
public sealed class ConfirmModal(string message) : ModalBase<bool>
{
    public override IRenderable Render()
    {
        return new Align(
            new Rows(
                new Markup($"{message}\n"),
                new Markup("[dim]Press [b]Enter[/] to confirm or [b]Esc[/] to cancel[/]")
            ),
            HorizontalAlignment.Center,
            VerticalAlignment.Middle
        );
    }


    public override void OnInput(InputAction action)
    {
        switch (action)
        {
            case InputAction.Confirm:
                Complete(true);
                break;
            case InputAction.Back:
                Complete(false);
                break;
        }
    }
}