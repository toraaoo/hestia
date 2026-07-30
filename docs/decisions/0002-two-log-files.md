# Two log files, because one file cannot be both readable and complete

*Applies to: [Cross-cutting foundations](../architecture/common.md)*

A single sink forced a choice every debug session lost: at trace it filled with
dependency chatter (`latest.log` was 90% `mio::poll` plus a `daemon.status` poll
every two seconds), and below trace it dropped the detail a bug needed. The
split is Forge's — a filtered `latest.log` for reading and a `debug.log`
firehose for reconstructing — and it is only possible per *target*, so the sinks
carry their own `EnvFilter` as per-layer filters rather than sharing one global
one. The firehose is deliberately unfiltered, inheriting the global filter
instead: it must take everything, dependencies included. Rotation moved to
`flexi_logger` rather than staying hand-rolled, which retired four bugs in the
old `rolling.rs` — a failed rotation re-gzipped the whole file on every
subsequent write, a failed archive then truncated the log it had failed to save,
archives were written non-atomically, and pruning sorted by mtime so an
unreadable archive was deleted first. Only the writer is borrowed: the
subscriber, formatting and spans stay `tracing`'s, because `flexi_logger`'s own
`trc::setup_tracing` installs a single-writer subscriber that cannot express
three differently-filtered sinks.
