using Hestia.Core.Abstractions;

namespace Hestia.Core.Jre;

public sealed record JreDetectedEvent(JavaRuntime Runtime) : HestiaEventBase;
public sealed record JreInstalledEvent(JavaRuntime Runtime) : HestiaEventBase;
public sealed record JreRemovedEvent(string RuntimeId) : HestiaEventBase;
