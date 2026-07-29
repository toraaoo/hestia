# No Service-class-per-prefix — but one registrar function per domain

*Applies to: [The daemon](../architecture/daemon.md)*

Unlike the historical C++ tree (which had one `Service` *object* per
channel-prefix, with its own lifetime and state), a handler here is a closure
and the registry is a flat map from channel to closure. What a domain gets is
only a `register(&mut Channels)` function: a compile-time grouping, no runtime
entity. The grouping exists because the flat `make_router()` grew to ~75
channels in one 1100-line function, which is the aggregation-point smell, not a
design: wiring in a channel is still exactly one `handle::<C>` line, now in the
file that owns its domain.
