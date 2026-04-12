using System.Collections.Concurrent;

namespace Hestia.Tui.Services;

internal sealed class UiDispatcher
{
    private readonly ConcurrentQueue<Action> _queue = new();

    public void Post(Action action) => _queue.Enqueue(action);

    public void Drain()
    {
        while (_queue.TryDequeue(out var action))
            action();
    }
}
