# A local-file import is inspected, not trusted

*Applies to: [Content & modpacks](../architecture/content.md)*

A picked path used to be copied blind under whatever kind the browse chip
happened to show — a resourcepack staged under the "mods" chip landed in `mods/`
and silently broke the game, and a `.mrpack` passed the filename check to
install as a garbage mod. The daemon now reads the archive's central directory
(`content/inspect.rs`, the `zip` crate already in-tree) and classifies it:
`content.inspect(path)` returns the detected kind, validity, and a reason.
Detection is **loader-agnostic** — a mod is any loader's manifest
(`fabric.mod.json`, `quilt.mod.json`, `META-INF/mods.toml`,
`neoforge.mods.toml`, …), so a new flavor is one more entry in the manifest
table, not a code path; a datapack vs. resourcepack is disambiguated by the
`data/` vs `assets/` tree under a shared `pack.mcmeta`. The desktop inspects
each file on pick, defaults its kind to the detected one, and the **review step
carries a per-file kind override** (constrained to the kinds the target accepts)
plus the install destination — so the detected kind is a suggestion, not a
verdict. On `content.add` the daemon hard-rejects only what genuinely cannot be
single-file content (an unreadable archive or a modpack) and otherwise **honors
the requested kind**, so a review override installs where asked; a
requested/detected mismatch is logged, not blocked. An unrecognised but valid
zip is installable once the user picks a kind. This is a desktop/daemon surface
(the CLI passes an explicit `--kind` already); the `.mrpack` extension is
dropped from the picker, since a modpack installs at instance-create, not here.
