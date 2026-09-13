# A pack syncs by identity, not by file — and only where it fits

*Applies to: [Instances](../architecture/entries.md#sync--shared-settings-across-instances)*

Resource packs and data packs are the two things people expect to follow them
between instances, and neither is a file two copies of which can be merged. A
pack is an installed *thing*: a project, at a version, enabled or not, in a load
order. So the unit shares that identity and nothing else.

A shared pack is a project id, the source it came from, and its file's hash. A
pass over a participating instance installs a pack it does not have through the
ordinary content pool — the same managed directory, provenance and mirror every
other install uses ([0013](0013-managed-dir-of-record.md)) — and propagates
enable, disable and removal by identity. **A version change never propagates**:
one instance updating a pack is not a statement about the others, which may be on
a game version that release does not support.

**Compatibility is the pack's own declaration, and it is a gate rather than a
failure.** A resource pack states the `pack_format` it was built for in its
`pack.mcmeta`, and the client states the format it reads in its own
`version.json`; the pack reaches an instance only when those agree. That is a
stronger test than the version list the platform publishes, which describes the
release rather than the archive, and it works for a pack installed from a file
that has no platform metadata at all. The platform's game versions and loaders
still decide *which* release to install. An instance that does not qualify is
skipped silently — it is not an error, and it must not be reported as one every
launch.

**A pack the instance did not choose does not travel.** Content carrying a
modpack origin belongs to that pack's configuration, not to the player's
library, so it is never lifted into the shared selection — the same rule that
keeps a modpack's `options.txt` from redefining everyone's settings.

Load order and enablement live in `options.txt`'s `resourcePacks` key, which is
never shared as a raw string: it names files by path, and a path means nothing on
an instance that does not hold that pack. The pack unit reconciles that
selection **per pack**, mapping each shared pack to the path it has locally, and
leaves entries for packs only that instance has exactly where they are.

**Datapacks keep the world as their record** ([0016](0016-datapacks-are-world-of-record.md)).
A datapack loads from inside a world, so syncing one cannot mean writing into
every save an instance holds — a world is the player's, and a pack appearing in
one they did not ask for is a change to how that world runs. The unit shares the
library and applies it when a world is created or when the user says so, never
retroactively. This is the narrow reading of a feature Modrinth ships switched
off for the same reason: their schema carries data packs and the capability
refuses them, because propagating into existing worlds is the only behaviour that
would make the toggle mean anything, and it is the one that cannot be undone.

**Rejected:** copying pack files between instances directly. It would duplicate
the pool, lose provenance, and leave the copies unupdatable — the content pool
already knows how to install a project by identity. **Also rejected:** syncing
versions along with identity, which would drag every instance onto whichever one
updated last, including instances that cannot run that release.
