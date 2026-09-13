# A world describes itself; a directory listing does not

*Applies to: [Servers & instances](../architecture/entries.md)*

`instance.worlds` began as a `read_dir` of `data/saves/` returning folder names,
which is all the datapack picker it was written for needed. But a folder name is
not the world: the player names a world in-game, so `saves/New World (2)` may be
"Hardcore Attempt 4", and only the save knows which version wrote it, how it
plays, or when it was last opened. So a world is read from its own `level.dat`
(gzipped NBT, via `fastnbt`) — `minecraft/world.rs` — and `WorldInfo` carries
the display name, version, game mode, difficulty, hardcore and cheat flags, last
played, footprint, and the world's own `icon.png`.

Two rules keep it honest. **The folder stays the identity**: every operation
still addresses a world by folder (a datapack installs into
`data/saves/<folder>/datapacks/`), because that is what the game reads and what
the content index keys on — the display name is presentation, and two worlds may
share one. And **every field but the folder is best-effort**: saves span more
than a decade of formats, an old one carries no `Version`, a corrupt or
half-written one cannot be parsed, and a world a running game is flushing may be
caught mid-write. None of that may hide a world from a listing, so a failure
yields the folder alone with `read: false`, and a front-end says "could not be
read" rather than rendering defaults as facts. The icon is **inlined as base64**
rather than served as a path: the alternative is widening the webview's
asset-protocol scope to the data home, which also holds `accounts.json`.
