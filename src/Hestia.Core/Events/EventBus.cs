using System.Diagnostics.CodeAnalysis;
using System.Threading.Channels;
using Hestia.Core.Abstractions;

namespace Hestia.Core.Events;

public sealed class EventBus : IEventBus, IDisposable
{
    private readonly ReaderWriterLockSlim _lock = new(LockRecursionPolicy.NoRecursion);
    private readonly Dictionary<Type, List<SubscriptionEntry>> _subscriptions = new();
    private bool _disposed;

    public ValueTask PublishAsync<TEvent>(TEvent @event, CancellationToken ct = default)
        where TEvent : IHestiaEvent
    {
        ArgumentNullException.ThrowIfNull(@event);

        _lock.EnterReadLock();
        try
        {
            if (!_subscriptions.TryGetValue(typeof(TEvent), out var entries))
                return ValueTask.CompletedTask;

            foreach (var entry in entries)
                entry.Channel.Writer.TryWrite(@event);
        }
        finally
        {
            _lock.ExitReadLock();
        }

        return ValueTask.CompletedTask;
    }

    public ChannelReader<TEvent> Subscribe<TEvent>()
        where TEvent : IHestiaEvent
    {
        var channel = Channel.CreateBounded<IHestiaEvent>(new BoundedChannelOptions(256)
        {
            FullMode = BoundedChannelFullMode.DropOldest,
            SingleWriter = false,
            SingleReader = false,
            AllowSynchronousContinuations = false
        });

        var typedReader = new TypedChannelReader<TEvent>(channel.Reader);
        var entry = new SubscriptionEntry(channel, typedReader);

        _lock.EnterWriteLock();
        try
        {
            if (!_subscriptions.TryGetValue(typeof(TEvent), out var list))
            {
                list = [];
                _subscriptions[typeof(TEvent)] = list;
            }
            list.Add(entry);
        }
        finally
        {
            _lock.ExitWriteLock();
        }

        return typedReader;
    }

    public void Unsubscribe<TEvent>(ChannelReader<TEvent> reader)
        where TEvent : IHestiaEvent
    {
        _lock.EnterWriteLock();
        try
        {
            if (!_subscriptions.TryGetValue(typeof(TEvent), out var list))
                return;

            var entry = list.FirstOrDefault(e => ReferenceEquals(e.TypedReader, reader));
            if (entry is null) return;

            list.Remove(entry);
            entry.Channel.Writer.TryComplete();
        }
        finally
        {
            _lock.ExitWriteLock();
        }
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _lock.EnterWriteLock();
        try
        {
            foreach (var entries in _subscriptions.Values)
                foreach (var entry in entries)
                    entry.Channel.Writer.TryComplete();
            _subscriptions.Clear();
        }
        finally
        {
            _lock.ExitWriteLock();
        }

        _lock.Dispose();
    }

    private sealed class SubscriptionEntry(Channel<IHestiaEvent> channel, object typedReader)
    {
        public Channel<IHestiaEvent> Channel => channel;
        public object TypedReader => typedReader;
    }

    private sealed class TypedChannelReader<TEvent>(ChannelReader<IHestiaEvent> inner)
        : ChannelReader<TEvent>
        where TEvent : IHestiaEvent
    {
        public override bool TryRead([MaybeNullWhen(false)] out TEvent item)
        {
            while (inner.TryRead(out var raw))
            {
                if (raw is TEvent typed)
                {
                    item = typed;
                    return true;
                }
            }
            item = default;
            return false;
        }

        public override ValueTask<bool> WaitToReadAsync(CancellationToken ct = default)
            => inner.WaitToReadAsync(ct);

        public override Task Completion => inner.Completion;
    }
}
