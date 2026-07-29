# Settings capture is opt-in per profile, and scopes only settings

*Applies to: [Content & modpacks](../architecture/content.md)*

An uncaptured profile inherits the global `shared/` store; `capture` snapshots
the settings-class sync targets into the profile's own store
(`<instance>/profiles/<name>/`, whose existence *is* the captured flag —
disk-is-the-registry, like `java` and `backups`) and from then on launches under
that profile sync against it. Divergence after capture is by design; `release`
deletes the dir and the profile inherits the global store again. Under linked
sync the two target classes capture differently: the `config` **folder repoints
the link** — `data/config` links into the profile store instead of the global
one, so in-game settings changes write through to the captured store and never
touch the global one — while `options.txt` keeps the per-scope
**copy-reconcile** with the same merge rules. `saves` and `screenshots` always
stay on the global store: capture forks *settings*, not game data (worlds stay
shared across profiles by construction). The stale-link relink handles every
scope switch, because a profile store path counts as a hestia store target
(`…/profiles/<name>/<rel>`); capture and release require a stopped instance — a
live session's `config` link writes through the store being replaced. A profile
rename moves its captured dir; a profile removal deletes it.
