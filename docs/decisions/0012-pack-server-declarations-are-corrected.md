# A pack's `env.server` is a claim, not a fact — so a server install corrects it

*Applies to: [Content & modpacks](../architecture/content.md)*

Trusting the index was the whole bug: Aged 3.1.2 declares 184 of its 212 mods
`server: required`, and fourteen of those are client-only (sodium, iris,
entityculling, lootbeams, skinlayers3d, welcomescreen and the rest). Fabric's
own environment check catches most, so a pack looks installed and then
misbehaves — which is exactly the failure a launcher must not leave to the user
to diagnose from a server log.

The rule and its list are `itzg/docker-minecraft-server`'s
(`FileInclusionCalculator`), adopted verbatim because that project has
maintained the only such list for years and a second, divergent one helps
nobody:

```text
include = force_include(path) || (env.server != unsupported && !exclude(path))
```

Matching is a case-insensitive substring of the whole path, so a bare `sodium`
covers every version; a force-include outranks everything, `env.server`
included, which is the escape hatch for a pack that under-declares.
`content/exclude/defaults.rs` is the table — Rust rather than a bundled JSON
asset, following `skins/defaults.rs`: it changes only when someone edits this
repository, so it belongs in source the compiler checks. The `modpack.*` config
keys layer a user's own lists over it and can switch it off entirely, and they
take itzg's own delimiters so a `MODRINTH_EXCLUDE_FILES` value pastes in
unchanged. What is held back rides on the result as
`WarningInfo::ModpackFilesExcluded`, naming each file: a mod silently missing is
the same invisible outcome the warning type exists to prevent.

**The correction is server-side only.** The list means "client mods a pack
wrongly called server-compatible"; applied to an instance it would strip out
precisely the mods a modpack is installed for.

Two rules that used to be hestia's own gave way to itzg's, since a divergence
here is a pack that installs differently for no stated reason. A pack file of a
kind the entry's flavor cannot *manage* is no longer dropped — it is written to
the path the pack named, so a server takes the pack's `resourcepacks/` whatever
hestia's own pool model does with it (`ModpackFilesNotAccepted` is gone with
it). And a **first** install writes the pack's overrides over whatever is on
disk, as docker-mc-server's `REPLACE_EXISTING` does; nothing there is the user's
yet, and refusing to overwrite `data/server.properties` the create had just
written meant a server pack could not ship its own. Only an *update* keeps the
hash guard and `ModpackOverridesKept` — by then an edited file genuinely is the
user's, which is a distinction itzg's stateless re-extraction cannot draw and
hestia's `modpack.json` record can.
