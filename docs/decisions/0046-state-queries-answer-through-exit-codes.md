# A state query answers through its exit code, not only its stdout

*Applies to: [Front-ends](../architecture/frontends.md)*

`hestia daemon status` printed `stopped` and exited 0, so `if hestia daemon
status; then …` was true whether or not the daemon was running — the exit code
conflated *answering* with *affirming*. Flipping that one command to exit 1
would have been worse: it collapses "not running" into "the query failed", which
is the distinction a script actually needs, and it leaves the next state verb
free to invent its own convention. So the contract is stated once
(`cli/src/exit.rs`, documented in ../cli.md) in systemd's vocabulary: **0**
did-what-was-asked / running, **3** answered and *not* running, **1** the
command failed, **2** usage (clap's own). `dispatch` therefore returns an
`ExitStatus` rather than `()`, and the two verbs that assert one subject's
running-ness — `daemon status`, `server <name> status` — produce it; everything
else maps to `Active`. Verbs that *describe* rather than assert (`info`, `sync
status`, the lists) stay 0 deliberately: "inactive" is not a claim they make,
and overloading them would make 3 meaningless.
