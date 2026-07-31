# Design decisions

Every non-trivial architectural choice in Hestia, recorded next to the reason it
was made — what changed, what it replaced, and what was rejected. The
[architecture](../architecture.md) pages describe *what the system is*; these
describe *why it is that*.

Each entry is one decision, numbered in the order it was written down, and links
back to the subsystem page it explains. Numbers are permanent: a superseded
decision is rewritten in place rather than renumbered, so an inbound link never
rots.

**Adding one:** when you make a call that a future reader would otherwise have to
reverse-engineer, add a file here (`NNNN-kebab-title.md`, next number free), open
with an *Applies to* line pointing at its subsystem page, and list it below. State
the alternative you rejected — that is usually the part nobody can recover later.

---

### [The socket boundary](../architecture/wire.md)

- **0001** — [The envelope seam fails closed, so no decode site can forget the version check](0001-envelope-fails-closed.md)
- **0029** — [A degraded outcome rides on the result, never only in the log](0029-degraded-outcomes-ride-on-the-result.md)
- **0030** — [A warning the user did not cause is a bug in the launcher, not a notice](0030-warnings-the-user-did-not-cause.md)
- **0031** — [Everything serialized is camelCase, except the `config.*` key vocabulary and upstream DTOs](0031-camelcase-except-the-config-vocabulary.md)
- **0035** — [A job is cancelled by asking, at safe checkpoints — never by disconnecting](0035-jobs-are-cancelled-by-asking.md)
- **0048** — [One event-callback slot per client `Session`](0048-one-event-callback-per-session.md)

### [Cross-cutting foundations](../architecture/common.md)

- **0002** — [Two log files, because one file cannot be both readable and complete](0002-two-log-files.md)
- **0003** — [A crash must survive the process that had no console](0003-crash-reports-survive-the-process.md)
- **0057** — [Materialised game files live under one `meta/` root](0057-meta-root-for-materialised-files.md)

### [The engine](../architecture/engine.md)

- **0027** — [A temp artifact is only valid while its job holds the claim — so a restart invalidates every one](0027-temp-artifacts-are-reclaimed-at-startup.md)
- **0036** — [Supervision is engine state, and one stop reaches the whole tree](0036-supervision-is-engine-state.md)
- **0037** — [Workloads outlive the daemon by design](0037-workloads-outlive-the-daemon.md)
- **0038** — [A finished process is labelled, not merely unrecorded](0038-a-finished-process-is-tombstoned.md)
- **0040** — [Following logs is scoped to the entry, not to one run of it](0040-following-logs-is-entry-scoped.md)
- **0058** — [News and notices are one mechanism with a severity dial, not two systems](0058-announcements-are-one-mechanism.md)
- **0064** — [A managed document carries its own schema version, and an unreadable one is set aside](0064-a-managed-document-carries-its-schema-version.md)

### [The daemon](../architecture/daemon.md)

- **0032** — [No Service-class-per-prefix — but one registrar function per domain](0032-one-registrar-per-domain.md)
- **0033** — [Instances are gated on a signed-in account, in the router](0033-instance-surface-gated-on-an-account.md)
- **0034** — [An aggregation point is a directory, not a file](0034-an-aggregation-point-is-a-directory.md)
- **0039** — [Stopping the daemon has three meanings; the front-end picks one, the wire carries two](0039-stopping-the-daemon-has-three-meanings.md)
- **0063** — [Discord presence belongs to the daemon, and is a loop rather than a hook](0063-discord-presence-is-a-daemon-loop.md)

### [Minecraft providers](../architecture/minecraft.md)

- **0004** — [NeoForge's game jar is built, not downloaded — so a flavor can install](0004-neoforge-builds-its-own-jar.md)
- **0005** — [A Paper build is a loader version, and Mojang orders the catalogue](0005-paper-build-is-a-loader-version.md)
- **0006** — [A flavor states what it needs, before the user commits to it](0006-flavor-states-its-requirements.md)
- **0007** — [Spigot and CraftBukkit are compiled here, because no one may ship them](0007-spigot-is-compiled-locally.md)
- **0008** — [What an entry takes is a property of its flavor, and the flavor says so](0008-flavor-declares-accepted-content.md)
- **0009** — [A flavor may recommend JVM flags; the user still outranks it](0009-flavor-recommends-jvm-flags.md)

### [Servers & instances](../architecture/entries.md)

- **0021** — [The entry root is hestia's; `data/` is the game's](0021-entry-root-versus-data-dir.md)
- **0022** — [Sync links folders and copies files — Pandora's split, adopted](0022-sync-links-folders-copies-files.md)
- **0023** — [The id is an opaque uuid; the directory is the slug — decoupled](0023-id-is-a-uuid-directory-is-a-slug.md)
- **0024** — [Backups follow docker-mc-backup, minus what the launcher already owns](0024-backups-follow-docker-mc-backup.md)
- **0025** — [A world describes itself; a directory listing does not](0025-a-world-describes-itself.md)
- **0026** — [An unfinished record says which kind of unfinished, so recovery can act on it](0026-server-phase-over-a-ready-bool.md)
- **0028** — [The properties schema is generated, not maintained — and it is not the file](0028-properties-schema-is-generated.md)
- **0041** — [An instance runs many sessions; a server runs one](0041-an-instance-runs-many-sessions.md)
- **0042** — [Per-session logs come from a generated Log4j2 config, not a captured pipe](0042-per-session-log4j-config.md)
- **0056** — [Server provisioning is front-loaded by design](0056-server-provisioning-is-front-loaded.md)
- **0059** — [The server console is RCON, not a stdin pipe](0059-the-console-is-rcon-not-a-pipe.md)
- **0062** — [Joining directly is a launch parameter, not a second launch path](0062-joining-directly-is-a-launch-parameter.md)

### [Content & modpacks](../architecture/content.md)

- **0010** — [Content is normalized behind one trait, following Prism's `ResourceAPI`](0010-one-content-provider-trait.md)
- **0011** — [A modpack is three things at once, and each goes where it already belongs](0011-modpack-decomposes-into-existing-parts.md)
- **0012** — [A pack's `env.server` is a claim, not a fact — so a server install corrects it](0012-pack-server-declarations-are-corrected.md)
- **0013** — [Installed content is managed-dir-of-record, mirrored into `data/`](0013-managed-dir-of-record.md)
- **0014** — [Enable/disable, update-check, and pin extend the same model](0014-enable-update-check-and-pin.md)
- **0015** — [A local-file import is inspected, not trusted](0015-local-imports-are-inspected.md)
- **0016** — [Datapacks are world-of-record, not managed-dir-of-record](0016-datapacks-are-world-of-record.md)
- **0017** — [A content profile is a selection, not a copy](0017-content-profile-is-a-selection.md)
- **0018** — [A global profile stores project references, never jars](0018-global-profile-stores-references.md)
- **0019** — [Settings capture is opt-in per profile, and scopes only settings](0019-profile-settings-capture.md)

### [Accounts & skins](../architecture/accounts.md)

- **0020** — [Skins follow Modrinth's shape, minus its couplings — and skip the CLI](0020-skins-follow-modrinth-minus-couplings.md)

### [Import & export](../architecture/transfer.md)

- **0061** — [An archive format is a module, not a branch — and the launcher matches on the recipe](0061-an-archive-format-is-a-module.md)

### [Front-ends](../architecture/frontends.md)

- **0043** — [Entry-first, with verb-first shortcuts for the hot path](0043-entry-first-cli-grammar.md)
- **0044** — [Every daemon capability gets a scriptable verb, or a written reason it has none](0044-every-capability-gets-a-verb.md)
- **0045** — [`-vv` buys wire visibility, not more volume](0045-vv-buys-wire-visibility.md)
- **0046** — [A state query answers through its exit code, not only its stdout](0046-state-queries-answer-through-exit-codes.md)
- **0047** — [Interaction is fullscreen; bare progress is one line](0047-fullscreen-interaction-inline-progress.md)
- **0049** — [The desktop bridge is one generic command, not a facade mirror](0049-desktop-bridge-is-one-generic-command.md)
- **0050** — [Messages are organised on one axis — where the string is rendered — and split one file per root](0050-messages-organised-by-render-surface.md)
- **0051** — [Sign-in is the one bespoke shell command — it must be](0051-sisu-sign-in-is-a-shell-command.md)
- **0052** — [Front-end preferences are desktop-local, in the data home — not the daemon](0052-desktop-prefs-live-in-the-data-home.md)
- **0053** — [Offline is one state, not a failure per read — and the shell brings its own daemon up](0053-offline-is-one-state.md)
- **0054** — [The daemon spawns the tray; the tray outlives the daemon](0054-the-daemon-spawns-the-tray.md)
- **0055** — [The tray and desktop must not share a GApplication id](0055-tray-and-desktop-app-ids.md)
- **0060** — [Job progress paints once per frame, because the store re-renders synchronously](0060-job-progress-paints-once-per-frame.md)
