# Changelog

Release notes, newest first. The release workflow reads the section matching
the version being cut into `latest.json`'s `notes`, so what the updater offers
is what is written here.

One `## <version>` heading per release. Everything until the next heading is
that release's notes, rendered as markdown.

## 1.0.0

The first release. A resident daemon owns everything, and the desktop app and
the `hestia` CLI are two views of the same state — so a server keeps running
when you close the window, and anything one front-end can do, the other can too.

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
- **Self-update** over a signed release feed, an in-app announcement feed, and
  a system tray beside the running daemon.

Known gaps: pre-1.19 clients launch without their LWJGL natives, and the
legacy (virtual) asset layout is not materialized — very old versions will not
launch correctly.
