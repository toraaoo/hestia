using Hestia.Tui.Input;
using Hestia.Tui.Modals;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.CreateServer;

public sealed class EnumPickerModal<T>(string title, T current) : ModalBase<T?>
    where T : struct, Enum
{
    private readonly T[] _values = Enum.GetValues<T>();
    private int _cursor = Array.IndexOf(Enum.GetValues<T>(), current);

    public override IRenderable Render()
    {
        var table = new Table()
            .HideHeaders()
            .Border(TableBorder.None)
            .Expand()
            .AddColumn(new TableColumn("").Width(3))
            .AddColumn(new TableColumn(""));

        for (var i = 0; i < _values.Length; i++)
        {
            var sel = i == _cursor;
            table.AddRow(
                new Markup(sel ? "[cyan]>[/]" : ""),
                new Markup(sel ? $"[bold green]{_values[i]}[/]" : $"[dim]{_values[i]}[/]"));
        }

        return new Panel(new Rows(
            table,
            new Rule().RuleStyle(Style.Parse("dim")),
            new Markup("[dim] [b]↑↓[/] navigate · [b]Enter[/] select · [b]Esc[/] cancel[/]")
        ))
            .Header($"[bold] {Markup.Escape(title)} [/]")
            .Border(BoxBorder.Rounded)
            .BorderColor(Color.Green)
            .Expand();
    }

    public override void OnInput(InputAction action)
    {
        switch (action)
        {
            case InputAction.MoveUp:
                _cursor = Math.Max(0, _cursor - 1);
                break;
            case InputAction.MoveDown:
                _cursor = Math.Min(_values.Length - 1, _cursor + 1);
                break;
            case InputAction.Confirm:
                Complete(_values[_cursor]);
                break;
            case InputAction.Back or InputAction.Quit:
                Complete(null);
                break;
        }
    }
}
