# Materialised game files live under one `meta/` root

*Applies to: [Cross-cutting foundations](../architecture/common.md)*

The data home holds what a user would recognise as theirs (`instances/`,
`servers/`, `accounts.json`, `config.json`), the launcher's internals (`cache/`,
`logs/`, `processes/`), and the `java/` runtimes; the game files the launcher
materialises at launch — `versions/`, `libraries/`, `assets/`, `natives/` — sit
under `meta/`. This is the Modrinth (Theseus) layout; Prism-style root-level
sprawl buries the user's own directories among derived, re-downloadable ones.
`meta/` is also one obvious unit to reclaim: everything under it is regenerated
on demand. Natives are per-version (`meta/natives/<version>`), not per-instance,
so the instance directory stays a pure game dir.
