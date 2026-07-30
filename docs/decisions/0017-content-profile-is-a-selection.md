# A content profile is a selection, not a copy

*Applies to: [Content & modpacks](../architecture/content.md)*

An instance's profiles (`profiles.json` beside `content.json`; absent = no
profiles) are named subsets of the managed pool, keyed by **filename** — the one
index field always present and unique (`project_id` is empty for local imports).
The managed dirs stay the single source of truth: activating a profile changes
only what the launch-time reconcile mirrors into `data/` — members are mirrored,
tracked non-members have their `data/` copy removed (the managed copy stays),
and untracked files are never touched, consistent with the untracked-not-adopted
rule. No profile active = mirror everything — exactly the pre-profile behavior,
so existing instances need no migration. Selectable kinds are mods,
resourcepacks, and shaders only: a datapack *is* world data, outside the pool.
Worlds, `servers.dat`, and all other game data are shared across profiles *by
construction* — every profile runs against the same single `data/` (per-profile
game dirs and symlinked game dirs were rejected). The pool keeps profiles honest
at its edges: removing content prunes the filename from every profile, and a
content update remaps a member to the new version's filename. When sessions are
already running, a launch skips the reconcile entirely (the mirror is in use;
jars are locked on Windows) and a profile override that differs from the active
one is refused. The `none` name is reserved: `launch { profile: "none" }`
overrides an active profile with "no profile" for one launch. Servers have no
profiles.
