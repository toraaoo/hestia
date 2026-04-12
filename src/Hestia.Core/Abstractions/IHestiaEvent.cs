namespace Hestia.Core.Abstractions;

public interface IHestiaEvent
{
    DateTimeOffset OccurredAt { get; }
}

public abstract record HestiaEventBase : IHestiaEvent
{
    public DateTimeOffset OccurredAt { get; init; } = DateTimeOffset.UtcNow;
}
