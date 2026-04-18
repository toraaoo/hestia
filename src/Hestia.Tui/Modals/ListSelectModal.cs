using Hestia.Tui.Input;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Modals;

public readonly struct ListSelectResult<T>
{
    private readonly T _value;

    public bool HasValue { get; }

    public T Value => HasValue
        ? _value
        : throw new InvalidOperationException("ListSelectResult has no value.");

    private ListSelectResult(T value)
    {
        _value = value;
        HasValue = true;
    }

    public static ListSelectResult<T> None => default;
    public static ListSelectResult<T> Some(T value) => new(value);
}

public sealed class ListSelectModal<T> : ModalBase<ListSelectResult<T>>
{
    private readonly string _title;
    private readonly Func<T, string> _display;
    private readonly Func<CancellationToken, Task<List<T>>>? _loader;
    private readonly (string Label, Func<T, bool> Predicate)[] _filters;
    private readonly bool[] _filterEnabled;

    private List<T> _all = [];
    private List<T> _filtered = [];
    private string _search = "";
    private int _cursor;
    private int _offset;
    private bool _loading;
    private string? _error;

    public ListSelectModal(
        string title,
        IReadOnlyList<T> items,
        Func<T, string> display,
        (string Label, Func<T, bool> Predicate)[]? filters = null)
    {
        _title = title;
        _display = display;
        _filters = filters ?? [];
        _filterEnabled = new bool[_filters.Length];
        Array.Fill(_filterEnabled, true);
        _all = items.ToList();
        ApplyFilters();
    }

    public ListSelectModal(
        string title,
        Func<CancellationToken, Task<List<T>>> loader,
        Func<T, string> display,
        (string Label, Func<T, bool> Predicate)[]? filters = null)
    {
        _title = title;
        _loader = loader;
        _display = display;
        _filters = filters ?? [];
        _filterEnabled = new bool[_filters.Length];
        Array.Fill(_filterEnabled, true);
        _loading = true;
    }

    protected override async Task OnShowAsync(CancellationToken ct)
    {
        if (_loader is null) return;
        try
        {
            _all = await _loader(ct);
            ApplyFilters();
        }
        catch (Exception ex)
        {
            _error = ex.Message;
        }
        finally
        {
            _loading = false;
        }
    }

    public override bool OnRawKey(ConsoleKeyInfo key)
    {
        if (key.Key == ConsoleKey.Backspace)
        {
            if (_search.Length == 0) return false;
            _search = _search[..^1];
            ApplyFilters();
            return true;
        }

        if (char.IsControl(key.KeyChar)) return false;
        _search += key.KeyChar;
        ApplyFilters();
        return true;
    }

    public override void OnInput(InputAction action)
    {
        switch (action)
        {
            case InputAction.Back or InputAction.Quit:
                Complete(ListSelectResult<T>.None);
                break;
            case InputAction.Confirm when _filtered.Count > 0:
                Complete(ListSelectResult<T>.Some(_filtered[_cursor]));
                break;
            case InputAction.MoveUp:
                MoveCursor(-1);
                break;
            case InputAction.MoveDown:
                MoveCursor(1);
                break;
            case InputAction.Tab when _filters.Length > 0:
                _filterEnabled[0] = !_filterEnabled[0];
                ApplyFilters();
                break;
        }
    }

    private void MoveCursor(int dir)
    {
        var windowSize = WindowSize();
        _cursor = Math.Clamp(_cursor + dir, 0, Math.Max(0, _filtered.Count - 1));
        if (_cursor < _offset)
            _offset = _cursor;
        else if (_cursor >= _offset + windowSize)
            _offset = _cursor - windowSize + 1;
    }

    private void ApplyFilters()
    {
        var source = _all.AsEnumerable();
        for (var i = 0; i < _filters.Length; i++)
            if (_filterEnabled[i])
                source = source.Where(_filters[i].Predicate);
        _filtered = string.IsNullOrEmpty(_search)
            ? source.ToList()
            : source.Where(x => _display(x).Contains(_search, StringComparison.OrdinalIgnoreCase)).ToList();
        var windowSize = WindowSize();
        _cursor = Math.Clamp(_cursor, 0, Math.Max(0, _filtered.Count - 1));
        _offset = Math.Clamp(_offset, 0, Math.Max(0, _filtered.Count - windowSize));
    }

    private static int WindowSize() => Math.Max(3, Console.WindowHeight - 10);

    public override IRenderable Render()
    {
        var fieldWidth = Math.Clamp(Console.WindowWidth - 40, 20, 50);
        var searchContent = string.IsNullOrEmpty(_search)
            ? "(type to filter)"
            : _search;
        var visibleSearch = searchContent.Length > fieldWidth - 1
            ? searchContent[^(fieldWidth - 1)..]
            : searchContent;
        var paddedSearch = visibleSearch.PadRight(fieldWidth - 1);
        var searchText = Markup.Escape(paddedSearch) + "[blink]_[/]";

        var header = new Table().HideHeaders().NoBorder().Collapse()
            .AddColumn(new TableColumn("").NoWrap());
        header.AddRow(new Markup($"[dim]Search:[/] {searchText}"));

        var countInfo = _filtered.Count > 0
            ? $"[dim]{_filtered.Count}"
              + (string.IsNullOrEmpty(_search) ? "" : $" of {_all.Count}")
              + " items[/]"
            : "  ";

        var filterLabel = BuildFilterHint();
        var footer = new Markup(
            $"{countInfo} · [dim]↑↓ nav · [b]Enter[/] select · [b]Esc[/] cancel · [b]Tab[/][/] {filterLabel}"
        );

        var body = RenderBody();

        var root = new Layout("Root")
            .SplitRows(
                new Layout("Content"),
                new Layout("Footer").Size(1)
            );

        var content = new Panel(new Align(
                new Rows(
                    new Markup($"[bold]{Markup.Escape(_title)}[/]"),
                    new Markup(""),
                    header,
                    new Markup(""),
                    body
                ),
                HorizontalAlignment.Center,
                VerticalAlignment.Middle
            ))
            .Border(BoxBorder.None)
            .Expand();

        root["Content"].Update(content);
        root["Footer"].Update(new Align(footer, HorizontalAlignment.Center));
        return root;
    }

    private string BuildFilterHint()
    {
        if (_filters.Length == 0) return "";
        var f = _filters[0];
        var state = _filterEnabled[0] ? "Show" : "Hide";
        return $"[{(_filterEnabled[0] ? "dim green" : "dim")}]{state} {Markup.Escape(f.Label)}[/]";
    }

    private IRenderable RenderBody()
    {
        if (_loading)
        {
            return new Markup("[dim]Loading…[/]");
        }

        if (_error is not null)
        {
            return new Markup($"[red]{Markup.Escape(_error)}[/]");
        }

        if (_filtered.Count == 0)
        {
            return new Markup(_all.Count == 0 ? "[dim]No items.[/]" : "[dim]No matches.[/]");
        }

        var table = new Table().HideHeaders().NoBorder().Collapse()
            .AddColumn(new TableColumn("").NoWrap().Centered());

        var end = Math.Min(_offset + WindowSize(), _filtered.Count);
        for (var i = _offset; i < end; i++)
        {
            var item = _filtered[i];
            var sel = i == _cursor;
            var label = _display(item);

            var combined = Markup.Escape(label);

            var labelMarkup = new Markup(sel
                ? $"[bold green reverse]{combined}[/]"
                : $"[white]{combined}[/]");

            table.AddRow(labelMarkup);
        }

        return table;
    }
}