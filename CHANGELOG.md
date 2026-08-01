# Changelog

Release notes, newest first. The release workflow reads the section matching
the version being cut into `latest.json`'s `notes`, so what the updater offers
is what is written here.

One `## <version>` heading per release. Everything until the next heading is
that release's notes, rendered as markdown.

## 0.0.1

First numbered build of the all-Rust workspace.

- **Servers and instances** for vanilla, Fabric and NeoForge, plus Paper,
  Folia, Spigot and CraftBukkit on the server side. A server is fully
  provisioned at create, so starting it never waits on the network.
- **Content and modpacks** from Modrinth — mods, plugins, resource packs,
  shaders and datapacks, with dependencies resolved and a `data/` mirror that
  survives a backup restore.
- **Backups** for servers, on demand or scheduled, taken live under the RCON
  save-off dance.
- **Shared settings** across instances: `options.txt` merged, worlds and
  configs linked into one store.
