# Datapacks are world-of-record, not managed-dir-of-record

*Applies to: [Content & modpacks](../architecture/content.md)*

The [managed-dir model](0013-managed-dir-of-record.md) exists so content
survives a `data/` swap on backup restore — but a datapack *is* `data/`: it
loads from inside a world (`data/<level-name>/` for a server,
`data/saves/<world>/` for an instance), which the world backup already captures.
So a datapack has no managed copy and no mirror; it installs straight into its
world's `datapacks/`, `sync` skips it (the world archive restores it), and
remove/untracked are world-aware. A server has one world (`level-name`, read
from `server.properties`); an instance has many, so the install names one or
more — repeatable `--world`, or an interactive multi-select over
`instance.worlds`. The index keys a datapack by world, so the same one coexists
across several worlds; a removal clears every copy unless narrowed to named
worlds (`remove --world`, or the session's pre-checked world list when
unchecking a multi-world pack). The client-side support flag is waived for
datapacks: they run on a world's server side, including a client's integrated
server, so a source marking a datapack client-unsupported must not block
installing it on an instance. With `saves/` linked (linked sync), an instance's
datapack lives in the *shared* world every instance opens — the pack itself is
visible everywhere, while its `content.json` provenance stays in the installing
instance, so other instances list it as untracked world data. Known behavior,
not a bug: the world carries its own datapacks, and exactly one instance manages
each.
