# The entry root is hestia's; `data/` is the game's

*Applies to: [Servers & instances](../architecture/entries.md)*

A server or instance directory used to *be* the game's working directory, which
left hestia nowhere to put its own artifacts without mixing them into files the
game owns and rewrites. Splitting the tree gives each side a clean namespace:
`data/` is exactly what the game reads and writes (the launch plan's cwd — jar,
world, saves, logs), and the root holds the record beside the managed content
directories the upcoming mod/plugin/config/backup management will populate
(`mods/`, `plugins/`, `resourcepacks/`, `configs/`, `backups/`). Directories
appear on demand rather than at create, so a tree only shows what is actually in
use. The layout change is not migrated: pre-`data/` entries must be recreated
(or their game files moved into `data/` by hand).
