# Changelog

Release notes, newest first. The release workflow reads the section matching
the version being cut into `latest.json`'s `notes`, so what the updater offers
is what is written here.

One `## <version>` heading per release. Everything until the next heading is
that release's notes, rendered as markdown.

## 1.0.0-beta.2

Two Linux fixes found on SteamOS.

- **The tray no longer crashes** on desktops that ship no appindicator library
  (GNOME without the extension, SteamOS). It reached the status area through a
  library that panics when absent, and the daemon spawns it on every start, so
  each one wrote a crash report. A tray is optional — where none can be shown,
  it now exits quietly and everything else carries on.
- **The AppImage renders again.** It shipped its own `libwayland-client` and put
  it ahead of the host's, so a system whose graphics stack expects a newer one
  failed to start the webview and the window came up blank. That library now
  comes from the host, where it belongs.

## 1.0.0-beta.1

The first beta of the 1.0 release. A resident daemon owns everything, and the
desktop app and the `hestia` CLI are two views of the same state — so a server
keeps running when you close the window, and anything one front-end can do, the
other can too.

- **Instances** for vanilla, Fabric and NeoForge. Launch several sessions of
  one instance at once, start straight into a world or onto a server, and move
  an instance between game versions in either direction.
- **Servers** for vanilla, Fabric, NeoForge, Paper, Folia, Spigot and
  CraftBukkit — fully provisioned at create, with their own jar, Java runtime,
  EULA and port, live resource charts, and a console over RCON.
- **Content** from Modrinth or CurseForge, a pasted project link, or a local
  file: mods, plugins, resource packs, shaders and datapacks, with dependencies
  resolved. Update, pin, enable and remove in batches, and slice what is
  installed into named profiles — per instance or global.
- **Modpacks** installed into a new or an existing entry, with an update check
  against the pack's published versions.
- **Backups** for servers, on demand or scheduled, taken live under the RCON
  save-off dance and restored from a stamped manifest.
- **Import and export** — Hestia archives, `.mrpack`, and Prism / MultiMC
  instances.
- **Accounts and skins** — Microsoft sign-in with token rotation, a skin
  library with a real-time 3D preview, the vanilla characters, and your capes.
- **Shared settings** across instances: `options.txt` merged, worlds and
  configs linked into one store, and existing instances adopted into it.
- **Old versions** launch as they should: a pre-1.19 client's LWJGL natives are
  unpacked from their classifier jars, and an asset index predating the hashed
  store is mirrored to the named tree those clients read.
- **Self-update** over a signed release feed, an in-app announcement feed, and
  a system tray beside the running daemon.

This build follows the beta feed, and lands on `1.0.0` by itself once that
ships.
