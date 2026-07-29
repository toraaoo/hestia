# Sync links folders and copies files — Pandora's split, adopted

*Applies to: [Servers & instances](../architecture/entries.md)*

Sync was originally all-copy ("copied, not symlinked"): each instance kept its
own physical copy of every target, reconciled newest-wins at launch. That call
was revisited for one reason — worlds. Copying `saves/` across instances would
duplicate gigabytes per instance and still leave each copy divergent; linking
stores a world **once** and shares it instantly. So folder targets (`saves`,
`config`, `screenshots`) are now **links** into the flat `shared/` store (a
symlink on POSIX, a junction on Windows — junctions need no privileges), while
file targets (`options.txt` key-merged, `servers.dat`) keep the copy-reconcile:
file symlinks need elevation or developer mode on Windows, and merge semantics
need a real copy anyway. The original decision's three objections each found a
narrower home instead of blocking linking wholesale: concurrent live servers →
servers are decoupled from sync entirely (a server's shareable state is its own
`server.config.*` and `server.properties`, never a cross-entry store); content
ownership → the managed content dirs are still rejected as targets (per-instance
selection is impossible over a shared dir); backups archiving through links →
instance backups no longer exist. The safety story was Pandora's
**empty-or-linked guard**: a folder became a link only when missing, empty, or
already linked into a hestia store — a non-empty real directory was never
touched, only surfaced as `cannot_link` until an explicit `sync adopt` moved its
entries into the store (all-or-nothing per target, refused on any name
collision). That guard is now **narrowed to the collision it was really about**
— see [Warnings the user did not
cause](0030-warnings-the-user-did-not-cause.md): a folder holding only the
instance's own files is adopted automatically, since moving it can destroy
nothing, and only a name the store already has stops it. Only links pointing
into a hestia store (`…/shared/<target>`) are ever touched, so a user's own
symlinks survive; a stale store link after a data-home move is relinked at the
next launch. Pack selection (`options.txt`'s `resourcePacks`) stays entry-local
— merged like Pandora's, but never pushed to the store. **Accepted risks,
documented not guarded:** two instances (or sessions) opening one shared world
are arbitrated only by Minecraft's own `session.lock`, and instances of
different versions/loaders writing one world can corrupt it — plus, until
import/export lands, instance data (the shared worlds store included) has no
backup story at all. Any code that walks or deletes an instance's `data/` must
treat a link as a boundary, never a directory to descend into —
`remove_dir_all`'s link-preserving behavior is pinned by a test.
