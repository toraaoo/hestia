# The daemon spawns the tray; the tray outlives the daemon

*Applies to: [Front-ends](../architecture/frontends.md)*

`hestiad` spawns the tray on every serve (detached, like every workload), so the
tray is simply always there when the daemon is — including a login autostart. It
deliberately does *not* die with the daemon: a stopped daemon is exactly when
the tray is most useful (the greyed status plus a start action), so only its own
Quit removes it. The spawn is best-effort and unconditional — including under a
`HESTIA_SOCK` override, since the spawned tray inherits the variable and follows
its daemon's endpoint (the dev scripts run on a dev endpoint by design; a tray
gated on the default endpoint would vanish exactly where the daemon is exercised
most, and a hand-started one would control the wrong daemon). Only a headless
session (no `DISPLAY`/`WAYLAND_DISPLAY` on Linux), a missing binary, or
`HESTIA_NO_TRAY=1` (how the e2e test keeps its throwaway daemons quiet) means no
tray. A duplicate spawn after a daemon restart is absorbed by the tray itself:
it takes an exclusive lock keyed by its endpoint in the transport runtime dir
(flock on POSIX, a no-sharing open on Windows) and exits at startup when another
instance holds it — per-endpoint, so a dev daemon's tray and the session's tray
coexist.
