using Hestia.Tui.Input;
using Hestia.Tui.Modals;
using Hestia.Tui.Navigation;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.Samples;

/// <summary>
/// Demonstrates list navigation, screen push, and modal usage.
/// ↑↓ navigate · Enter → <see cref="SampleDetailScreen"/> · D → <see cref="ConfirmModal"/> · Esc back
/// </summary>
public sealed class SampleScreen(INavigator navigator, Func<string, SampleDetailScreen> detailFactory) : ScreenBase
{
    private readonly List<string> _items = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"];
    private int _cursor;
    private string? _status;
    private Layout? _layout;

    public override IRenderable Render()
    {
        _layout ??= new Layout("Root").SplitRows(
            new Layout("Content"),
            new Layout("Footer").Size(1)
        );

        var table = new Table()
            .Expand()
            .Border(TableBorder.Simple)
            .AddColumn(new TableColumn("Item"));

        foreach (var (item, i) in _items.Select((x, i) => (x, i)))
        {
            var active = i == _cursor;
            table.AddRow(active ? $"[bold cyan]▶  {item}[/]" : $"   {item}");
        }

        if (_items.Count == 0)
            table.AddRow("[dim](no items)[/]");

        _layout["Content"].Update(
            new Panel(table).Header("[bold]Sample Screen[/]").BorderColor(Color.Blue).Expand()
        );

        _layout["Footer"].Update(
            _status is not null
                ? new Markup($" [green]{_status}[/]")
                : new Markup("[dim] [b]↑↓[/] navigate · [b]Enter[/] detail · [b]D[/] delete · [b]Esc[/] back[/]")
        );

        return _layout;
    }

    public override void OnInput(InputAction action)
    {
        _status = null;

        switch (action)
        {
            case InputAction.MoveUp:
                _cursor = Math.Max(0, _cursor - 1);
                break;

            case InputAction.MoveDown:
                _cursor = Math.Min(_items.Count - 1, _cursor + 1);
                break;

            case InputAction.Confirm when _items.Count > 0:
                navigator.Push(detailFactory(_items[_cursor]));
                break;

            case InputAction.Delete when _items.Count > 0:
                var target = _items[_cursor];
                navigator.ShowModal(
                    new ConfirmModal($"Delete '{target}'?"),
                    confirmed =>
                    {
                        if (!confirmed) return;
                        _items.RemoveAt(_cursor);
                        _cursor = Math.Clamp(_cursor, 0, Math.Max(0, _items.Count - 1));
                        _status = $"Deleted '{target}'";
                    });
                break;

            case InputAction.Back:
            case InputAction.Quit:
                navigator.Pop();
                break;
        }
    }
}
