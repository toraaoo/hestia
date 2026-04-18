using Hestia.Core.Minecraft.Models;
using Hestia.Core.Minecraft.Providers;
using Hestia.Tui.Input;
using Hestia.Tui.Modals;
using Spectre.Console;
using Spectre.Console.Rendering;

namespace Hestia.Tui.Features.CreateServer;

public sealed class VersionPickerModal(ServerType type) : ModalBase<MinecraftVersion?>
{
    private List<MinecraftVersion> _all = [];
    private List<MinecraftVersion> _filtered = [];
    private string _search = "";
    private int _cursor;
    private int _offset;
    private bool _showSnapshots;
    private bool _loading = true;
    private string? _error;

    private const int WindowSize = 18;

    protected override async Task OnShowAsync(CancellationToken ct)
    {
        try
        {
            IProvider provider = type switch
            {
                ServerType.Fabric => new FabricProvider(),
                _                 => new VanillaProvider(),
            };
            _all = await provider.GetVersionsAsync();
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
        _cursor = 0;
        _offset = 0;
        return true;
    }

    public override void OnInput(InputAction action)
    {
        switch (action)
        {
            case InputAction.Back or InputAction.Quit:
                Complete(null);
                break;
            case InputAction.Confirm when _filtered.Count > 0:
                Complete(_filtered[_cursor]);
                break;
            case InputAction.MoveUp:
                MoveCursor(-1);
                break;
            case InputAction.MoveDown:
                MoveCursor(1);
                break;
            case InputAction.Tab:
                _showSnapshots = !_showSnapshots;
                ApplyFilters();
                break;
        }
    }

    private void MoveCursor(int dir)
    {
        _cursor = Math.Clamp(_cursor + dir, 0, Math.Max(0, _filtered.Count - 1));
        if (_cursor < _offset)
            _offset = _cursor;
        else if (_cursor >= _offset + WindowSize)
            _offset = _cursor - WindowSize + 1;
    }

    private void ApplyFilters()
    {
        var source = _showSnapshots ? _all : _all.Where(v => !v.IsSnapshot).ToList();
        _filtered = string.IsNullOrEmpty(_search)
            ? source
            : [.. source.Where(v => v.Version.Contains(_search, StringComparison.OrdinalIgnoreCase))];
        _cursor = Math.Clamp(_cursor, 0, Math.Max(0, _filtered.Count - 1));
        _offset = Math.Clamp(_offset, 0, Math.Max(0, _filtered.Count - WindowSize));
    }

    public override IRenderable Render()
    {
        var rows = new List<IRenderable>();

        var searchText = string.IsNullOrEmpty(_search)
            ? $"[dim](type to search)[blink]_[/][/]"
            : Markup.Escape(_search) + "[blink]_[/]";
        var snapLabel = _showSnapshots ? "[green]snapshots on[/]" : "[dim]snapshots off[/]";
        rows.Add(new Markup($"  Search: {searchText}    {snapLabel}  [dim][b]Tab[/] toggle[/]"));
        rows.Add(new Rule().RuleStyle(Style.Parse("dim")));

        if (_loading)
        {
            rows.Add(new Markup("  [dim]Fetching versions…[/]"));
        }
        else if (_error is not null)
        {
            rows.Add(new Markup($"  [red]{Markup.Escape(_error)}[/]"));
        }
        else if (_filtered.Count == 0)
        {
            rows.Add(new Markup(_all.Count == 0
                ? "  [dim]No versions available.[/]"
                : "  [dim]No matches.[/]"));
        }
        else
        {
            var table = new Table()
                .HideHeaders()
                .Border(TableBorder.None)
                .Expand()
                .AddColumn(new TableColumn("").Width(4))
                .AddColumn(new TableColumn(""))
                .AddColumn(new TableColumn("").Width(14));

            var end = Math.Min(_offset + WindowSize, _filtered.Count);
            for (var i = _offset; i < end; i++)
            {
                var v = _filtered[i];
                var sel = i == _cursor;
                var marker = sel ? "[bold cyan]»[/]" : " ";
                var label  = sel ? $"[bold green]{Markup.Escape(v.Version)}[/]" : Markup.Escape(v.Version);
                var tag    = v.IsSnapshot ? "[dim](snapshot)[/]" : "";
                table.AddRow(new Markup(marker), new Markup(label), new Markup(tag));
            }

            rows.Add(table);
        }

        rows.Add(new Rule().RuleStyle(Style.Parse("dim")));

        var countInfo = _filtered.Count > 0
            ? $"[dim]{_filtered.Count}"
              + (string.IsNullOrEmpty(_search) ? "" : $" of {_all.Count}")
              + " versions[/]"
            : "";
        rows.Add(new Markup($"  {countInfo}  [dim][b]↑↓[/] navigate · [b]Enter[/] select · [b]Esc[/] cancel[/]"));

        return new Panel(new Rows(rows))
            .Header($"[bold] Pick Version ({type}) [/]")
            .Border(BoxBorder.Rounded)
            .BorderColor(Color.Green)
            .Expand();
    }
}
