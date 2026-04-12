using Hestia.Core.Abstractions;

namespace Hestia.Core.Monitoring;

public sealed record PlayerJoinedEvent(Guid ServerId, PlayerInfo Player) : HestiaEventBase;
public sealed record PlayerLeftEvent(Guid ServerId, PlayerInfo Player) : HestiaEventBase;
public sealed record ServerStatusUpdatedEvent(Guid ServerId, ServerStatus Status) : HestiaEventBase;
