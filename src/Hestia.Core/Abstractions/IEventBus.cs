using System.Threading.Channels;

namespace Hestia.Core.Abstractions;

public interface IEventBus
{
    ValueTask PublishAsync<TEvent>(TEvent @event, CancellationToken ct = default)
        where TEvent : IHestiaEvent;

    ChannelReader<TEvent> Subscribe<TEvent>()
        where TEvent : IHestiaEvent;

    void Unsubscribe<TEvent>(ChannelReader<TEvent> reader)
        where TEvent : IHestiaEvent;
}
