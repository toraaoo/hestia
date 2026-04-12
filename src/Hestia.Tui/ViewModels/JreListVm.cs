using Hestia.Core.Abstractions;
using Hestia.Tui.Formatting;

namespace Hestia.Tui.ViewModels;

internal sealed class JreListVm
{
    private readonly IHestiaService _service;

    public List<string> Rows { get; private set; } = [];
    public event Action? Changed;

    public JreListVm(IHestiaService service) => _service = service;

    public async Task RefreshAsync(CancellationToken ct = default)
    {
        var runtimes = await _service.GetRuntimesAsync(ct);
        Rows = runtimes.Count == 0
            ? ["  (no JREs found)"]
            : runtimes.Select(RowFormatters.JreRow).ToList();
        Changed?.Invoke();
    }
}
