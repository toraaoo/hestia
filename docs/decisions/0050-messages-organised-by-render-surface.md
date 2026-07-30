# Messages are organised on one axis — where the string is rendered — and split one file per root

*Applies to: [Front-ends](../architecture/frontends.md)*

The catalogue (paraglide/inlang, `frontend/messages/`) had grown to 705 keys in
a single 800-line file under 43 top-level namespaces that mixed four different
axes at once: feature (`skins`, `news`, `sync`), widget type (`label`, `action`,
`tab`, `status`), domain enum (`kind`, `flavor`, `gamemode`), and wire shape
(`error`, `warning`) — plus catch-alls (`common`, `resources`, one key apiece)
and an `entry`/`entry_settings` split that existed only because flat naming had
run out of room. Nothing said where a new string went, so nothing kept two
people from putting it in two places: the same vocabulary was defined four times
over (`gamemode.survival` and `worlds.mode_survival`, `difficulty.*` and
`worlds.difficulty_*`), 60 keys had outlived their call sites, and pt-BR was 42
behind with nothing checking it.

There are now four roots and one rule for each. **`app.*`** is shell chrome and
shared vocabulary (nav, window, action, label, status, toast, search, time,
jobs, validation, daemon). **One root per feature** — `library`, `entry`,
`server`, `instance`, `content`, `profile`, `skin`, `settings`, `news`,
`account` — mirroring `frontend/src/features/`, so the string a component
renders lives in the file named after that component's directory (`entry.*`
holds what servers and instances share: the create wizard, per-entry settings,
the stop dialogs). **`domain.*`** is the proto-mirrored enums — content kinds,
flavors, gamemodes, difficulties, provision phases, entry types — which is what
retires the duplicate tables: a gamemode is worded once, whether it is rendered
in the create wizard or read off a world's `level.dat`. And
**`error.*`/`warning.*`** keep the shape the wire gives them (`kind`, `code`,
`token`, `hint`), because they are looked up dynamically from an `ErrorInfo`
variant and a front-end must not restate the daemon's vocabulary.

Underscores stop standing in for nesting (`entry_settings.remove_instance_title`
→ `entry.settings.remove.instance_title`), which also closes a latent collision:
paraglide flattens a dotted key to underscores, so `entry_settings.x` and
`entry.settings_x` compile to one identifier. inlang's `pathPattern` takes an
array, so each root is its own file (`messages/{locale}/app.json`, …) — a
feature's strings are one file to open rather than a region of an 800-line one,
and two features' translations no longer collide in the same diff.

**The guard is a test, not a convention** (`frontend/tests/messages.test.ts`):
every locale must cover the base locale exactly and interpolate the same
`{placeholders}`, every key the source references must exist, and every defined
key must have a call site. The last one only works if dynamic lookups are
declared, so `DYNAMIC_PREFIXES` lists the tables reached by
``m[`error.kind.${kind}`]`` and friends — adding an undeclared one fails the
dead-key test deliberately, since an undeclared table is exactly what makes dead
keys unprovable. The one merge that needed code rather than data was
`error.token.{server,instance,profile}`, which duplicated `entry.type_*` in the
same register; `errors.ts` now resolves a token through an ordered
`TOKEN_TABLES` (`domain.entry_type.` then `error.token.`) so the word is written
once. The rest of the token table is deliberately *not* merged into
`domain.kind.*`: it is keyed by wire enum spelling (`data_pack`) and worded as a
mid-sentence fragment ("mod"), where the UI label is title-case ("Mod") — so
they are homographs, not duplicates, and folding them together would trade a
data duplicate for a lookup indirection.
