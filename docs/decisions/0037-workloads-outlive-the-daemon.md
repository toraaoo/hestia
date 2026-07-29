# Workloads outlive the daemon by design

*Applies to: [The engine](../architecture/engine.md)*

The supervisor originally spawned children with `kill_on_drop` and piped output,
which killed every server and game session on a graceful daemon stop — and
leaked them untracked on a crash. Now the daemon is restartable/upgradable under
live workloads (the same reason Docker grew `live-restore`): stopping a workload
is always an explicit act (`server stop`, `process.stop`, `hestia daemon stop
--all`), never a side effect of daemon lifetime. The cost is honest bookkeeping
— on-disk records, start-time identity checks, file-based logs — and one
observable gap: an adopted process's exit code is unknowable.
