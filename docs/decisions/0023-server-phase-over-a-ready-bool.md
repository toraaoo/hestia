# An unfinished record says which kind of unfinished, so recovery can act on it

*Applies to: [Servers & instances](../architecture/entries.md)*

A server is registered before provisioning starts, because the record is what
holds its port claim through a long download; `provision_server` removes it if
the pipeline fails. But that cleanup only runs while the daemon that started the
create is alive — kill it mid-create and the half-registered record survived
every subsequent start, `start` correctly refusing it ("still provisioning")
while nothing ever reconciled or removed it. A permanent un-startable orphan,
holding a port.

The fix is not "register later": the claim genuinely needs the record first. It
is that `ready: bool` could not say what recovery needs to know. A record now
carries a **`ServerPhase`** — `Provisioning`, `Ready`, `Updating` — and
`Engine::recover()` reconciles at startup beside `ProcessSupervisor::recover()`
and the temp-artifact reclaim: no job survives a restart, so a `Provisioning`
record belongs to a create that will never finish and is **discarded**, reaching
the same conclusion the live failure path does.

The distinction earns the enum. `Updating` is *also* not-ready, but it belongs
to a server that was ready before — its world is on disk, so discarding it would
destroy real data. It is kept and logged, and updating again finishes it, which
is what the update path's own gate already promised. A single boolean conflated
"nothing here is yours yet" with "your world is here, mid-swap"; recovery cannot
be safe without telling them apart, and a test pins both outcomes.
