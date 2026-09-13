# Front-end preferences are desktop-local, in the data home — not the daemon

*Applies to: [Front-ends](../architecture/frontends.md)*

UI state (a dismissed first-run overlay, remembered view) is the front-end's
concern, not the launcher's, so it never crosses the socket: the
`prefs_list|set|remove` commands (`commands/prefs.rs`) read and write
`<data_home>/prefs.json` directly, resolving the same data home the engine uses
(`common::paths`, so `--home`/`$HESTIA_HOME`/the persisted pointer are
honoured). This keeps UI state out of the engine's typed `config` store (which
the CLI and every front-end would then see) and out of the webview's
`localStorage` (wiped with the webview cache, and not a real file). A Tauri
store plugin was rejected for keeping its own file in the app dir — an extra
indirection when a direct write to the data home is simpler. The store is
schema-less: the front-end owns its own keys, consumed through the frontend's
`usePrefs` hook.
