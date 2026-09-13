# Offline is one state, not a failure per read — and the shell brings its own daemon up

*Applies to: [Front-ends](../architecture/frontends.md)*

With no daemon running, every query failed, and three defaults turned that into
a hot loop: `retry: false` leaves a query in `error`, TanStack's `retryOnMount`
refetches it whenever a new observer mounts, and a refetch with no data reads as
`pending` — which flips a page's `loading`, swaps its body for the skeleton,
unmounts those observers, and remounts them to start again. Measured at ~150 IPC
calls a second and ~600 renders a second on the library page, which is what the
"flickering" was. Three changes settle it, one per layer: `retryOnMount: false`
(recovery is the reconnect sweep in `queries/invalidation.ts`, not a mount); the
bridge answers `connection_lost` from its held state while the daemon is down,
so a burst of reads costs one socket attempt per watch interval instead of one
each, and emits `hestia:connection` only on transitions; and the failures log at
`debug` rather than `warn` on both sides, since an offline daemon is a state the
status bar reports, not an error per call. The UI then says it once: an
`OfflineOverlay` in the app shell carrying the daemon's own start action — the
whole app is backed by the daemon, so there is nothing useful to do behind it.
The shell also **starts the daemon at launch** (`bridge::start`) — opening the
desktop is as deliberate a launch of Hestia as `hestia daemon start`, so the
overlay is the exception, not the greeting. Reconnection still never spawns: a
daemon stopped *during* a session was stopped on purpose.
