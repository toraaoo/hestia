# Sync is a closed catalogue of merged files — no links, no worlds

*Applies to: [Servers & instances](../architecture/entries.md)*

Sync shares a **closed catalogue** of four units: the game options, the
multiplayer list, the command history and the creative hotbars. Each names one
file whose format the launcher understands well enough to **merge** two edits
of, and that is what decides membership. A file that can only be copied whole
makes the second instance to write lose its change, so it does not belong in the
catalogue — which is why an arbitrary user-supplied path is not a sync target.

Every unit is copied into the instance and merged back out. Nothing is linked.
A link earns its keep only for a directory too large to duplicate — a worlds
store — and it charges for that: a guard deciding when a directory may become a
link, a state for one that already holds files, a migration to move them in, and
a rule that every walk of an instance's `data/` treats a link as a boundary.
Worlds stay with the instance that plays them, and none of that is needed to
share a few kilobytes of settings.

What a unit shares is not all-or-nothing. The options merge resolves per key, so
a key can be pinned local launcher-wide or on a single instance; a pinned key is
carried through untouched on both sides, because pinning on one instance must
not change what the others read. `servers.dat` merges per entry for the same
reason, keyed by the row's own name so that editing an address reads as an edit
rather than as a delete plus an add.

A unit is **off until it is enabled with a named source**. The shared copy has to
start as someone's, and a launcher that picks for itself defines everyone's
settings from whichever instance happens to launch first. Enabling seeds from one
named instance and records every instance's current content as its baseline, so
the first pass settles the shared copy's way. With several candidates and no
source named, the daemon refuses and hands back the candidates.

**Rejected:** folder targets for `config/` and `screenshots/`. A directory tree
has no merge — it needs per-file baselines and deletion semantics for the two
targets that least repay them, and a modpack's config tree belongs to the
instance anyway. **Also rejected:** an arbitrary-path escape hatch beside the
catalogue. It can only work by whole-file copy, which is the lost-edit behaviour
the catalogue exists to avoid, and it invites paths (`mods/`, `saves/`) that must
never be shared.
