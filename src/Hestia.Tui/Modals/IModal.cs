namespace Hestia.Tui.Modals;

public interface IModal<TResult>
{
    Task<TResult> ShowAsync(CancellationToken ct);
}
