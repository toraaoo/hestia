# A finished process is labelled, not merely unrecorded

*Applies to: [The engine](../architecture/engine.md)*

Two promises here could not both hold: "a terminal state keeps its logs for
post-mortem" and "a startup sweep deletes recordless dirs". A finished process
leaves *exactly* a recordless dir — `records::remove` drops `record.json` and
leaves the logs — so the sweep destroyed the post-mortem at the next restart,
and the guarantee survived only until then. The observable symptom was the odd
one: a killed process lingered in `process.list` (in memory only) until an
unrelated daemon restart, and that same restart deleted its logs. There was no
state in which the list was clean *and* the logs existed.

The bug was the sweep's criterion. "Has no `record.json`" was standing in for
"is a stray", but it is also the normal resting state of a finished process. So
the end is now **explicit on disk**, the disk-is-the-registry discipline used
for java runtimes, backups and server records: on exit the record is replaced by
a tombstone (`exit.json`: state, exit code, when it ended, and where its logs
are, since that is not derivable once the spec is gone). The sweep then deletes
only directories with **neither** marker — a true stray, hand-made or
half-written — and `TOMBSTONE_KEEP` prunes the oldest finished ones, because
"keep the logs" without stated retention means "grow forever" (count-based, as
`backup prune` is).

`process.list`/`status`/`logs` read terminal entries from the tombstones rather
than from memory, so what the daemon reports about a finished process no longer
depends on whether it has restarted since — including its logs, which is the
whole reason for keeping the directory. A process that died while unsupervised
is entombed by `recover()` at the moment it is noticed. The rejected
alternatives were exempting process dirs from the sweep (they then accumulate
forever, which is what the sweep exists to prevent) and adding a `process.clean`
verb (asking the user to resolve a contradiction the daemon should not have).
