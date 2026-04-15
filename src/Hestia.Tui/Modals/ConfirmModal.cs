using Hestia.Tui.Input;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Modals;

/// <summary>
/// Full-screen confirmation dialog. Returns true if the user confirmed.
/// Demonstrates how to implement <see cref="ModalBase{TResult}"/>.
/// </summary>
public sealed class ConfirmModal : ModalBase<bool>
{
    private readonly string _message;

    public ConfirmModal(string message) => _message = message;

    public override IRenderable Render() =>
        Align.Center(new Panel(
            new Markup($"[yellow]{_message}[/]\n\n[dim]Press [b]Enter[/] to confirm · [b]Esc[/] to cancel[/]")
        )
        {
            Border = BoxBorder.None,
            Header = new PanelHeader("[bold red]Confirm[/]"),
            Padding = new Padding(2, 1),
            Expand = true
        });

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
