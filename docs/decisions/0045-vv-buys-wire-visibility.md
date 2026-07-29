# `-vv` buys wire visibility, not more volume

*Applies to: [Front-ends](../architecture/frontends.md)*

The CLI advertised three verbosity levels but `cli` and `client` contained zero
`trace!` statements, so `-v` and `-vv` emitted byte-identical output — a flag
the binary could not honour. The fix was not to sprinkle `trace!` until the line
count differs: that satisfies a test while leaving the level meaning "more,
somehow". A verbosity level should buy a *capability*, and there is exactly one
thing a client can show that the daemon's own logs cannot — **the wire**. So
`-vv` is frames: each request and reply with its channel, correlation id, byte
size and round-trip time, plus session open/close and the count of waiters a
close woke. That is precisely what someone debugging a CLI-versus-daemon
disagreement needs, it lives in one place (`client/src/session.rs`), and it is
the same stream for every front-end that links `client`. Payloads are
deliberately **not** logged — they carry access tokens and rcon passwords — so a
frame reports its size, never its contents.
