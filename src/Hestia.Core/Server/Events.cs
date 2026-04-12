using Hestia.Core.Abstractions;

namespace Hestia.Core.Server;

public sealed record ServerCreatedEvent(MinecraftServer Server) : HestiaEventBase;
public sealed record ServerStartingEvent(Guid ServerId) : HestiaEventBase;
public sealed record ServerStartedEvent(Guid ServerId, int ProcessId) : HestiaEventBase;
public sealed record ServerStoppingEvent(Guid ServerId) : HestiaEventBase;
public sealed record ServerStoppedEvent(Guid ServerId, int ExitCode) : HestiaEventBase;
public sealed record ServerCrashedEvent(Guid ServerId, string Reason, int ExitCode) : HestiaEventBase;
public sealed record ServerDeletedEvent(Guid ServerId) : HestiaEventBase;
