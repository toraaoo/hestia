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
        var root = new Layout("Root")
            .SplitRows(new Layout("Content"));

        var content = new Panel(
                new Align(
                    new Markup($"{message}\n\n[dim]Press [b]Enter[/] to confirm · [b]Esc[/] to cancel[/]"),
                    HorizontalAlignment.Center,
                    VerticalAlignment.Middle
                )
            )
            .Border(BoxBorder.None)
            .Expand();


        root["Content"].Update(content);

        return root;
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