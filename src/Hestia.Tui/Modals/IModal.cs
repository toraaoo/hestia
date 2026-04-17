using Hestia.Tui.Input;

namespace Hestia.Tui.Modals;

public interface IModal<TResult>
{
    Task<TResult> ShowAsync(KeyMap keyMap, CancellationToken ct);
}
