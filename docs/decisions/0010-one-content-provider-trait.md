# Content is normalized behind one trait, following Prism's `ResourceAPI`

*Applies to: [Content & modpacks](../architecture/content.md)*

Prism Launcher drives Modrinth and CurseForge through a strategy-pattern
`ResourceAPI` whose results are platform-agnostic structs, so its UI never
special-cases a platform; Hestia adopts the same shape (`ContentProvider` +
`proto::content`) — and the same split as its own `minecraft` registry, so the
codebase has one way of saying "pluggable upstream catalogue". Resolution is
deliberately separate from installation: `modpack.resolve` returns a plain file
manifest (path, URL, checksum, client/server side) rather than writing anything,
because installing must compose with the entry stores' layout and locking
(`data/` vs the managed `mods/`/`resourcepacks/` roots, the backup in-flight
keys) — that materialize step landed later (see [Installed content is
managed-dir-of-record](0013-managed-dir-of-record.md)), and the wire contract
did not change when it did.
