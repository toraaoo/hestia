# Architecture

The reference for Hestia: what exists today, where it lives, and the reasoning
behind the structure. Read this first; [contributing.md](contributing.md) has the
copy-and-adapt recipes for extending it, and [packaging.md](packaging.md) covers
release artifacts.

Hestia is an all-Rust cargo workspace of small path crates. It runs as a daemon
(`hestiad`) with thin clients — the CLI, the desktop shell, the tray — driving it
over a local socket. The launcher engine lives only in the daemon; a front-end
physically cannot reach it except over the wire.

## One daemon, many front-ends

Hestia is a single domain core — the `engine` — owned by the daemon, driven by
several front-ends that are each a thin client over the socket (a Unix domain
socket on POSIX, a named pipe on Windows):

| Front-end | Binary           | Crate     | Stack                 | State                      |
|-----------|------------------|-----------|-----------------------|----------------------------|
| CLI       | `hestia`         | `cli`     | clap + ratatui        | shipped                    |
| Desktop   | `hestia-desktop` | `desktop` | Tauri v2 + React/Vite | stock shell, not yet wired |
| Tray      | `tray`           | `tray`    | tray-icon + tao       | shipped                    |

The daemon (`hestiad`) is the resident core. The CLI is the first-class,
fully-wired front-end; the tray accompanies every serving daemon with quick
actions; the desktop is a scaffold (see
[Front-ends](#front-ends-cli-desktop-tray)).

## The crate graph

A single workspace (`crates/*`). The one-way arrows are enforced by cargo, not by
discipline: only `daemon` lists `engine` as a dependency, so a front-end **cannot**
reach launcher logic — `cargo tree -i engine` shows only `daemon`.

```
proto   → wire contracts + domain types (serde)                    leaf
ipc     → transport (unix socket / named pipe) + JSON envelope      leaf   → (tokio, libc)
common  → logging (tracing) + app identity + path resolution        leaf
client  → typed client SDK (Session + one facade per domain)       → proto, ipc, common
engine  → config·cache·download·java·accounts·minecraft·content    → proto, common          (daemon-only)
cli     → bin hestia          (clap + ratatui presentation)        → client, common, proto
daemon  → bin hestiad         (router, services, supervisor)       → engine, proto, ipc, common, client
desktop → bin hestia-desktop  (Tauri v2 shell)                     → (tauri)                 (+ frontend/)
tray    → bin tray            (tray-icon + tao)                   → client, common, ipc
```

- **`proto`** and **`ipc`** together form the socket boundary — the one seam the
  daemon and every client share. `proto` is the *what* (typed payloads), `ipc` is
  the *how* (framing + envelope). Neither knows anything launcher-specific.
- **`client`** re-exports `proto`, so a front-end depends only on `client` to get
  both the SDK and the domain types.
- **`daemon`** is the only crate that links `engine`. It also links `client`, but
  only so `hestiad ping` can talk to an already-running daemon.
- **`engine`** is daemon-internal domain logic — the equivalent of Tailscale's
  `LocalBackend`. It never links `ipc` or `client`; it does not know a socket
  exists.

### Tech stack

- **Rust** (edition 2021), a **cargo** workspace; `rustfmt` + `clippy -D warnings`
  kept clean, `cargo-deny` for licenses/advisories.
- [tokio](https://tokio.rs/) — async runtime (client + daemon transport).
- [serde](https://serde.rs/) / serde_json — the wire and persistence marshalling.
- [tracing](https://github.com/tokio-rs/tracing) — structured logging.
- [clap](https://github.com/clap-rs/clap) — CLI parsing; [ratatui](https://ratatui.rs/)
  — the CLI's terminal presentation layer.
- [reqwest](https://github.com/seanmonstar/reqwest) (rustls) — engine HTTP
  (downloader, Adoptium, Mojang/Fabric meta, Microsoft auth).
- [p256](https://github.com/RustCrypto/elliptic-curves) — Xbox proof-key ECDSA
  (one cross-platform impl; no OpenSSL/CNG split).
- `sha1`/`sha2`, `tar`+`flate2`, `zip` — in-process checksums and archive
  extraction (no shelling out to system tools).
- [Tauri v2](https://tauri.app/) + [React](https://react.dev/)/[Vite](https://vitejs.dev/)
  (built with [Bun](https://bun.sh/)) — desktop; its
  [tray-icon](https://github.com/tauri-apps/tray-icon) +
  [tao](https://github.com/tauri-apps/tao) crates — the system tray.

## The socket boundary

Every request crosses the same seam. Two crates own it.

### `proto` — the no-drift wire contract

`proto` is pure data: no I/O, no async, `serde` derive is the codec. Both sides of
the socket marshal through **one** definition per channel, so the daemon and every
client cannot disagree — a mismatch is a compile error, not a runtime surprise.

A **`Contract`** (`contract.rs`) names its channel once and pairs it with request
and response payload types:

```rust
pub trait Contract {
    const CHANNEL: &'static str;
    type Params: Serialize + DeserializeOwned;
    type Result: Serialize + DeserializeOwned;
}
```

An unsolicited daemon→client push is a **`Topic`** (the implementing type is its
own payload). `Empty` is the `{}` payload for channels that take or return
nothing. One module per domain: `app`, `health`, `daemon`, `config`, `cache`,
`download`, `java`, `accounts`, `process`, `server`, `instance`, `events` —
plus `minecraft`, the provider vocabulary (`Flavor`, `GameVersion`, `Artifact`,
the profiles, `ProvisionProgress`) the `server` and `instance` domains share,
and `content`, the normalized third-party content vocabulary (`ContentProject`
with its images, `ContentVersion`, the paginated `SearchQuery`/`SearchResult`,
`ResolvedModpack`) — a front-end never sees a platform's raw shape.
Adding a channel is a struct plus an `impl Contract` — see
[contributing.md](contributing.md).

### `ipc` — transport + envelope

`ipc` carries the bytes and nothing domain-specific:

- **transport** (`transport.rs`) — the platform socket (Unix domain socket /
  Windows named pipe), `bind`/`connect`, a length-framed `FrameReader`/
  `FrameWriter`, and `Peer` (the connection's verified identity; `uid` and
  `authorized()` on POSIX via `libc` peer credentials).
- **protocol** (`protocol.rs`) — the JSON envelope, encoded/decoded in exactly one
  place. A request is `{v, channel, payload, id?}`; a response is
  `{v, ok, payload | error, id?}`; an event is `{event, payload}`. `PROTOCOL_VERSION`
  is `1`; same-major only.
- **endpoint** (`endpoint.rs`) — where the socket lives. The **runtime dir** holds
  the ephemeral socket (`$XDG_RUNTIME_DIR/hestia/hestiad.sock`, else
  `/tmp/hestia-<uid>/…`; a named pipe on Windows) and is deliberately distinct from
  the engine's persistent data home. `HESTIA_SOCK` overrides it so tests and
  side-by-side daemons never collide.
- **errors** (`errors.rs`) — the error-code vocabulary (`BAD_REQUEST`, `NOT_FOUND`,
  `UNKNOWN_CHANNEL`, `HANDLER_ERROR`, …) and the client-facing `IpcError`.

## `common` — cross-cutting

UI-free, domain-free code linked by the daemon and every client:

- **`app`** — the application identity constants (`NAME`, `ID`, `VENDOR`,
  `CHANNEL`, `VERSION` from `CARGO_PKG_VERSION`): one source of truth every binary
  reads.
- **`logging`** — `init_logging(console LogLevel, Option<FileLog>)` configures the
  process `tracing` subscriber once and returns a `LogGuard`. Each sink has its
  own level: the console (stderr), plus an optional rotated, compressed file —
  fresh-per-run (`logs/latest.log`, the long-lived daemon's) or appended across
  runs and rotated by size (`logs/hestia.log`, shared by the short-lived CLI
  invocations, whose console stays at warnings/errors unless `-v`/`-vv` raise it).
- **`paths`** — data-directory resolution: `--home` → `$HESTIA_HOME` → a persisted
  pointer (`config set home`) → the platform default (`~/.hestia`, `%APPDATA%\Hestia`
  on Windows). **Debug builds** anchor the default at `<workspace>/.hestia` so
  development never touches the real per-user directory. Also `config_path`,
  `log_dir`, and `set_persisted_home`.

## `client` — the typed SDK

The one way a front-end drives the daemon. `Client::connect(auto_spawn)` opens a
connection (auto-spawning `hestiad` if it is not running and `auto_spawn` is set);
`connect_to(endpoint)` targets an explicit socket without spawning.

- **`Session`** (`session.rs`) — the connection core, private to the crate: one
  persistent, multiplexed connection whose background reader task fulfils pending
  requests by id and delivers events to an installed callback. `call::<C>()`
  marshals through the contract and returns the `proto` result directly;
  `try_call` maps a `not_found` to `None`; `call_with_timeout` overrides the 10 s
  default; `run_job` drives a long-running operation, forwarding its progress
  events and blocking until a done/error topic arrives.
- **facades** (`facades/`) — one struct per domain in its own module, reached
  through a `Client` accessor (`client.java().install(21, …)`), mirroring the
  engine's domain modules on the other side of the socket. Facade methods are
  one-liners over `Session`: `App`, `Daemon`, `Config`, `Cache`, `Java`,
  `Accounts`, `Process`, `Server`, `Instance`, `Content`. `facades/jobs.rs` holds
  the drivers the server and instance facades share — the backup and content jobs
  publish the same topics, disambiguated by job id.
- **spawn** (`spawn.rs`) — locates and launches the `hestiad` binary, then retries
  the connection until it is listening.

## `engine` — the launcher engine

Daemon-internal domain logic. **`Engine`** (`engine/mod.rs`) is the aggregate root:
the daemon constructs exactly one and threads it through every request handler. It
resolves the data directory once and owns each subsystem as a member behind a
getter — and owns *only* that: the cross-subsystem flows composed over the
subsystems live in `engine/flows/` (`server`, `instance`, `backup`, `content`),
one `impl Engine` block apiece. Adding a domain is a module, a member, and a getter
here — the single growth point, with no change to the daemon's serve loop.
`set_data_home()`
re-resolves the directory and `reload()`s every subsystem so a `config set home`
takes effect on the running daemon, not just the next start.

The subsystems behind the aggregate:

- **`config`** (`Config`, `Settings`) — the typed settings store. The schema is one
  `Settings` struct: a setting is a field with its default, persisted as JSON
  through serde. Internal code reads a `settings()` snapshot and writes through
  `update()`; the dotted-path `get`/`set` serve the `config.*` channels and reject
  unknown keys and type-mismatched values — the struct *is* the validation.
  (`Settings` is empty today; the only live keys are the reserved `home` and
  `autostart`, which the daemon routes to the path pointer and the login
  registration rather than the store.) `reload()` repoints it on a data-home
  change.
- **`download`** (`Downloader`) — streams a URL to disk through a `.part` temp file
  (via reqwest), hashing incrementally when a checksum is given and renaming into
  place only on success. Stateless — the daemon's `DownloadManager` constructs one
  per download. The incremental SHA-1/SHA-256 hasher is `checksum.rs`.
- **`cache`** (`Cache`) — a content-addressed store of verified downloads under
  `<data_home>/cache/<algorithm>/<hex[..2]>/<hex>`, keyed by checksum so a file
  fetched once (a JDK, a library) is reused regardless of URL. Hits are **re-hashed
  on the way out**, so a damaged blob is evicted and the fetch falls back to the
  network — the cache can speed things up but never corrupt them. Served over the
  `cache.*` channels.
- **`accounts`** (`Accounts`) — Minecraft accounts signed in through Microsoft,
  persisted with their tokens in `<data_home>/accounts.json` (owner-only on POSIX;
  tokens never leave the daemon). Both methods use the well-known Minecraft client
  id, so no per-distribution Azure app is needed. Sign-in is two steps —
  `begin_login()` returns what the user must act on and holds per-login state in an
  in-memory pending map; `complete_login()` drives it to a stored account. Both
  converge on the same signed tail — Xbox device token → sisu `/authorize` → XSTS →
  `launcher/login` → profile — which `access_token()`'s token rotation also runs:
    - **device_code** (the CLI default, no paste): returns a `user_code` +
      `verification_uri`, then polls the device-code grant until the user approves.
    - **sisu** (the embedded-browser flow, `account login --sisu`): mints an ECDSA
      P-256 proof key, runs PKCE sisu `/authenticate`, returns the Microsoft
      sign-in URL, and redeems the redirect's OAuth code.
      The HTTP steps are the private `accounts/microsoft.rs`; Xbox request signing (the
      proof key and the FILETIME-stamped `Signature` header) is `accounts/signing.rs` —
      one cross-platform `p256` implementation.
- **`java`** (`Java`, `JavaProvider`) — installs and tracks Java runtimes under
  `<data_home>/java/<vendor>-<major>/` beside a `runtime.json` record; listing
  scans the directory, so the disk is the registry. `JavaProvider` is the abstract
  catalogue seam; `adoptium` (Eclipse Temurin) is the default. `install()` runs the
  blocking pipeline — resolve → download (SHA-256-verified, via `Downloader`) →
  extract (`tar`+`flate2`, the `zip` crate on Windows; all in-process) → register —
  staging into a `.staging` dir and renaming into place so a failure leaves nothing
  behind. The async wrapper and `java.install.*` progress events live in the
  daemon's `JavaInstallManager`.
- **`minecraft`** (`Minecraft`) — the server and instance (client) provider
  registries. A *flavor* is a distribution (`vanilla`, `fabric`); a provider lists
  the game *versions* it supports and *resolves* a request into a launch profile —
  the full descriptor (`ServerProfile` / `InstanceProfile`: primary artifact,
  libraries, asset index, java major, main class, args) the launch pipeline
  consumes. Stateless (every result is fetched upstream), so it needs no data
  directory. Manifest parsing lives in `minecraft/meta/` (`mojang`, `fabric`).
  Two further modules are the launch pipeline over the profiles:
    - **`minecraft/materialize`** — idempotently ensures profile pieces on disk
      (skip-if-present): single jars, Maven-layout libraries under the shared
      `meta/libraries/` root, and the content-addressed asset store
      (`meta/assets/indexes/<id>.json` + `meta/assets/objects/<hh>/<hash>`), all
      SHA-verified through `Downloader` (a bounded number of concurrent fetches).
    - **`minecraft/launch`** — pure assembly of a **`LaunchPlan`**
      (program/args/cwd): classpath joining and Mojang `${placeholder}`
      substitution (auth, paths, names); no I/O.
    - **`minecraft/rcon`** — a minimal RCON client (the vanilla remote-console
      protocol over localhost TCP): connect + authenticate + one command per
      call. The server console's transport — see the decision note below.
- **`content`** (`Content`) — the third-party content provider registry: mods,
  modpacks, resourcepacks, shaders discovered on a *source* platform. The
  `ContentProvider` trait is the seam (search with pagination, project detail,
  version resolution filtered by loader/game version, and modpack resolution);
  `modrinth` is the shipped source, CurseForge is a future impl behind the same
  trait — adding a source is a new impl plus one line in `Content::new`, the
  same shape as `minecraft`'s flavor registry. Stateless, like `minecraft`.
  Every platform response is mapped into the normalized `proto::content` types
  at this boundary (projects carry `icon_url`/gallery images for the desktop
  UI); `resolve_modpack` fetches a version's `.mrpack`, reads its
  `modrinth.index.json` in-process (the `zip` crate over memory — pack indexes
  are references, not embedded jars), and returns the file manifest plus the
  loader the pack pins, rejecting parent-escaping file paths at the edge. A
  provider also recognises its own site's project/version page URLs
  (`parse_url`), so a pasted `modrinth.com/mod/…` link installs like a slug.
  `content/install` is the per-entry install half: a `content.json` index in
  the entry root records each installed item's provenance
  (`InstalledContent`: kind, source, project/version ids, filename, sha1, and —
  for datapacks — the world it lives in); the file itself lands in the managed
  kind directory (`<entry>/mods/`, `resourcepacks/`, `shaderpacks/` — the
  game's own load-dir names, so the mirror is symmetric) and is
  **mirrored** (hardlink, else copy) into the game dir's matching directory.
  **Datapacks are the exception** — they load from inside a world, not a flat
  dir, so a datapack installs *straight into* `data/<level-name>/datapacks/` (a
  server's single world) or `data/saves/<world>/datapacks/` (an instance's
  named save, picked interactively over `instance.worlds`) with no separate
  managed copy: it is world data, so the world's own backup already covers it,
  restore heals it for free, and `sync` skips it (see the decision note below).
  A platform install picks the newest compatible version (`pick_version`,
  filtered by the entry's game version and, for mods, its loader) and resolves
  required dependencies breadth-first; a direct URL or a local file import
  records `source: "file"`/a platform id with no version to update against.
  Servers take mods and datapacks; instances take mods, resourcepacks, shaders,
  and datapacks. `Engine` composes the flows
  (`add_server_content`/`add_instance_content`, list/remove/update) and a
  `sync` pass re-mirrors any missing managed file at every start/launch (below).
  Installing a modpack's files and `overrides/` is the remaining materialize
  step.
- **`sync`** (`Sync`) — shared settings/configs: a small set of game-relative
  files/folders (`options.txt` key-merged, `servers.dat`, `config/`) propagated
  across entries through a persistent `<data_home>/shared/` store. Copy-based,
  not symlinked: each entry keeps its own copy under `data/`, and `apply`
  reconciles it with the store newest-wins at every start/launch (hooked into
  `server_launch_plan`/`prepare_instance`, before the content re-mirror).
  Targets **and the store are kept separate per kind** (`shared/servers/`,
  `shared/instances/`, each with its own `targets.json` and defaults): a server
  syncs different files than a client and must not share its mod `config/` with
  one. The managed content dirs and `saves/` are rejected as targets at the edge
  — see the decision note below.
- **`servers`** / **`instances`** (`Servers`, `Instances`) — the persistent
  stores, one directory per entry beside a JSON record (`servers/<id>/server.json`
  holding the resolved profile snapshot; the disk is the registry, as with
  `java`). Each record also carries a `JavaSettings` (`minecraft/launch.rs`):
  the per-entry `memory` (one value driving both `-Xms`/`-Xmx`) and extra
  `jvm-args`, injected into the launch plan at each start/launch; a server
  record also carries a `BackupSettings` (`backup.rs`): the scheduled-backup
  `backup-interval` (m/h/d units, empty disables) and `backup-retention`. The
  `config_get/set/list` methods validate and persist them (servers also pass
  property keys through to `server.properties` — a set must name a key the
  server's own generated file carries, so a typo cannot silently drift the
  file; the hestia-managed ports/rcon keys are rejected — see the decision
  note below). An entry directory holds the record beside `data/`, the game's
  own working directory; the root is reserved for the managed content
  directories (`mods/` for servers / `mods/`, `resourcepacks/`, `shaderpacks/`
  for instances, `backups/`) and the `content.json` install index, each created on
  demand — see the decision note below:

  ```
  servers/<id>/               instances/<id>/
  ├── server.json             ├── instance.json
  ├── content.json            ├── content.json
  ├── mods/ backups/          ├── mods/ resourcepacks/
  │                           │   shaderpacks/ backups/
  └── data/                   └── data/
      jar, libraries/,            saves, options, logs,
      eula.txt,                   mods/ (mirror) —
      server.properties,          the game dir the client
      world, logs, mods/          writes into
  ```

- **`backup`** — entry backups: gzipped tar archives of an entry's `data/`
  under its `backups/`, named `<utc-stamp>-<kind>.tar.gz` (kind = `manual` /
  `scheduled` / `update`) — the disk is the registry, here too. Creation
  skips what the launcher re-materialises (the server jar, `libraries/`,
  `logs/`, `cache/` — docker-mc-backup's default exclude set — plus the managed
  content mirror `mods/`; instances skip `logs/` and the mirrors `mods/`,
  `resourcepacks/`, `shaderpacks/`) and writes through a `.part` temp file; restore
  extracts into a
  staging directory, carries the skipped names over from the current tree
  (they belong to the record's *current* version), and swaps — a failure
  leaves the current data untouched. `prune` keeps the newest N of one kind.
  Every pass reports per-file progress.

  A server's record also claims its **ports**: the game port at create (lowest
  free from 25565, or pinned via the create params) and its rcon console
  (port + random password) at first start. Claims are checked against every
  other record plus a live bind probe under one allocation lock, so concurrent
  servers can never collide; `ensure_start_config` reconciles them into
  `server.properties` (preserving user edits) before each spawn.
  An instance's heavyweight files
  live in the shared roots and materialise at launch. The `Engine` aggregate
  composes the cross-subsystem flows: `provision_server` (resolve → register →
  ensure the Java runtime, installing through the cache when missing → download
  files → generate `server.properties` → apply create-time config → mark
  ready, removing the record on failure), `server_launch_plan`,
  `server_command` (one console command over rcon), `create_instance`,
  `prepare_instance` (materialise java/client/libraries/assets, then assemble
  the plan for the signed-in account's rotated token), and the version moves
  `update_server` / `update_instance` (re-resolve the same flavor at another
  version, take an automatic `update`-kind backup of the existing data, and
  swap the record's profile — a server also re-materialises its files under
  the `ready` gate and regenerates its properties schema; an instance pays at
  the next launch). Both directions work; a downgrade must be allowed
  explicitly, and the direction is judged by position in the flavor's own
  newest-first catalogue, not by parsing version strings. The aggregate also
  composes the backup flows over the `backup` module: `backup_server` (a live
  server's world saving pauses over RCON around the archive — `save-off`,
  `save-all flush`, tar, `save-on`, with `save-on` retried even when
  archiving fails, exactly docker-mc-backup's sequence),
  `restore_server_backup`, their instance counterparts (instances archive
  only while stopped — no RCON to quiesce a client), and
  `prune_server_backups`; one backup *or* restore runs per entry at a time.
  Servers are fully provisioned at create so `start` is an immediate spawn;
  instances are records at create and pay at launch.

> **Content is normalized behind one trait, following Prism's `ResourceAPI`.**
> Prism Launcher drives Modrinth and CurseForge through a strategy-pattern
> `ResourceAPI` whose results are platform-agnostic structs, so its UI never
> special-cases a platform; Hestia adopts the same shape (`ContentProvider` +
> `proto::content`) — and the same split as its own `minecraft` registry, so
> the codebase has one way of saying "pluggable upstream catalogue". Resolution
> is deliberately separate from installation: `modpack.resolve` returns a plain
> file manifest (path, URL, checksum, client/server side) rather than writing
> anything, because installing must compose with the entry stores' layout and
> locking (`data/` vs the managed `mods/`/`resourcepacks/` roots, the backup
> in-flight keys) — that materialize step lands with mod management, and the
> wire contract does not change when it does.

> **Installed content is managed-dir-of-record, mirrored into `data/`.** A mod
> is written to the entry root's `mods/` (hestia's namespace) with its
> provenance in `content.json`, then hardlinked/copied into `data/mods/` (what
> the game loads). The managed copy — not the one in `data/` — is the source of
> truth, which pays off three ways: (1) a backup restore swaps `data/` but the
> managed dirs live outside it, so `mods/`/`resourcepacks/`/`shaderpacks/` are added
> to the backup exclude/preserve set and a `sync` pass re-mirrors them at the
> next start/launch (`server_launch_plan`, `prepare_instance`) — restore heals
> itself and archives stay world-focused; (2) provenance survives, so `update`
> knows each item's project and current version (Prism keeps the same metadata
> in packwiz TOML sidecars — same idea, one index file); (3) a hand-dropped jar
> in `data/mods/` is surfaced as *untracked* rather than silently adopted.
> Installs run through a `ContentManager` mirroring `BackupManager` (job id,
> per-entry in-flight key, `content.progress|done|error` topics) and are
> refused on a running entry (open jars lock on Windows; changes only apply at
> the next start) or during a backup/update.

> **Datapacks are world-of-record, not managed-dir-of-record.** The managed-dir
> model above exists so content survives a `data/` swap on backup restore — but
> a datapack *is* `data/`: it loads from inside a world (`data/<level-name>/`
> for a server, `data/saves/<world>/` for an instance), which the world backup
> already captures. So a datapack has no managed copy and no mirror; it installs
> straight into its world's `datapacks/`, `sync` skips it (the world archive
> restores it), and remove/untracked are world-aware. A server has one world
> (`level-name`, read from `server.properties`); an instance has many, so the
> install names one or more — repeatable `--world`, or an interactive
> multi-select over `instance.worlds`. The index keys a datapack by world, so
> the same one coexists across several worlds; a removal clears every copy
> unless narrowed to named worlds (`remove --world`, or the session's
> pre-checked world list when unchecking a multi-world pack).
> The client-side support flag is waived for datapacks: they run on a world's
> server side, including a client's integrated server, so a source marking a
> datapack client-unsupported must not block installing it on an instance.

> **The entry root is hestia's; `data/` is the game's.** A server or instance
> directory used to *be* the game's working directory, which left hestia
> nowhere to put its own artifacts without mixing them into files the game
> owns and rewrites. Splitting the tree gives each side a clean namespace:
> `data/` is exactly what the game reads and writes (the launch plan's cwd —
> jar, world, saves, logs), and the root holds the record beside the managed
> content directories the upcoming mod/plugin/config/backup management will
> populate (`mods/`, `plugins/`, `resourcepacks/`, `configs/`, `backups/`).
> Directories appear on demand rather than at create, so a tree only shows
> what is actually in use. The layout change is not migrated: pre-`data/`
> entries must be recreated (or their game files moved into `data/` by hand).

> **Shared settings/configs are copied, not symlinked.** Pandora shares files
> across instances by symlinking whole folders (`saves`, `config`, …) into one
> live directory. That model fights three things Hestia already decided: servers
> run concurrently (two live processes on one symlinked `config/`/world corrupt
> each other), the content system already owns `resourcepacks/`/`shaderpacks/`
> (a symlink there would leak one entry's content to all), and backups archive
> `data/` (a symlinked `saves` would be archived-through or clobbered on
> restore). So `sync` is **copy-based**: each entry keeps its own physical copy
> under `data/`, reconciled newest-wins with a persistent `shared/` store at
> every start/launch. Nothing is live-shared, so concurrent writers are safe and
> backups stay intact; the cost is that propagation is at-launch, not instant —
> which settings don't need. Scope is settings/config only: the managed content
> dirs and `saves/` are rejected as targets (the content system shares content;
> worlds belong to backups). Pack selection (`options.txt`'s `resourcePacks`)
> stays entry-local — merged like Pandora's, but never pushed to the store.
> Unlike Pandora (client-only), Hestia manages **servers and instances**, whose
> syncable files differ — a server has no `options.txt`, and its mod `config/`
> is not a client's — so targets and the physical store are split per kind
> (`shared/servers/` vs `shared/instances/`, each with its own defaults). The
> two never mix; there is deliberately no cross-kind sharing.

> **Rename re-slugs the id and moves the directory.** The `id` is not just a
> display alias — it is the directory name (`servers/<id>/`), the supervisor's
> process key (`server-<id>`), the port-claim and content in-flight key, and
> how the on-disk process records are keyed. So a rename cannot be a cheap
> field write: it re-derives the id from the new name (the same `slugify` as
> create), moves the entry directory, and rewrites the record, carrying the
> ports, rcon, JVM/backup settings, and all game data along untouched. It is
> refused unless the entry is stopped and free of any in-flight
> backup/update/content job (and, for a server, not still provisioning) —
> exactly the guards `remove` already uses, because both re-key or delete the
> tree a running process and its records point at. After the move the daemon
> `discard`s the old process id so no stale supervisor state survives under a
> key that no longer resolves. The alternative — a stable id with only the
> display name mutable — was rejected: it leaves the on-disk slug frozen at the
> original name, so `servers/smp/` lingers long after the server is called
> `cozy`, and the directory stops being a legible name for what it holds.

> **Backups follow docker-mc-backup, minus what the launcher already owns.**
> The reference behaviour (itzg/docker-mc-backup) is: pause world writes over
> RCON (`save-off`, `save-all flush`), tar the data, `save-on` guaranteed by
> an exit trap, timestamped `%Y%m%d-%H%M%S` gzip archives, exclude
> `*.jar,cache,logs`, prune on a schedule. Hestia keeps that shape and
> diverges where the launcher knows more than a sidecar can: excluded
> binaries (jar, `libraries/`) are *carried over* on restore rather than
> missing, because the record's profile — not the archive — says which
> version the entry runs; restore is a staged swap instead of an
> extract-into-empty-dir script; retention is count-based per kind, pruning
> only `scheduled` archives so a deliberate manual or pre-update backup is
> never auto-deleted; and the schedule lives on the server record
> (`backup-interval`/`backup-retention` config keys) rather than a sidecar's
> environment. Version updates always back up first — an update is the one
> moment data provably changes shape, and the confirmation gate (downgrade
> warnings) already marks it as risky. Instances get on-demand backups only:
> a Minecraft client has no RCON channel to quiesce it, so it must be stopped
> to archive, and an interactive client session has no analogue of a
> long-running server's unattended schedule.

> **The properties schema is generated, not maintained.** `config set`
> validates a `server.properties` key against the server's own file, written
> by the server itself during provisioning — not against a curated key list.
> A hand-kept list is a per-version maintenance liability (keys appear,
> disappear, and differ across the versions Hestia launches; the list would
> silently rot). Instead the create job runs the freshly downloaded server
> once *before* writing `eula.txt`: the EULA gate makes it emit a complete
> `server.properties` (every key + default for exactly that version, mods
> included) and exit almost immediately, before binding ports or generating a
> world. Pre-1.7.10 servers have no EULA gate and would boot for real, so the
> run is killed after a 60 s timeout. Generation failure is a warning, not a
> create failure — and a server with no file to validate against accepts any
> key rather than rejecting every key. A version update reruns the trick with
> `eula.txt` suspended (and rewritten after): the new server binary rewrites
> the file to exactly its version's schema, keeping set values and dropping
> keys it no longer knows.

Errors are `thiserror` enums (e.g. `ConfigError`); the daemon maps them to
`ipc::errors` codes at the service boundary. `anyhow` is used where an operation
composes many fallible steps (accounts, minecraft, java, provisioning).

## `daemon` — hestiad

The resident core: it owns the IPC endpoint, routes requests to handlers,
supervises launched processes, and manages autostart. The only crate that links
`engine`.

- **`main.rs`** — bootstrap only: clap parsing (`serve`, the default, `ping`, or
  `stop` — a graceful self-stop that leaves supervised processes running, letting
  the Windows installer quiesce the daemon without the optional CLI), logging
  init (a rotated file for the long-lived daemon; stderr for the one-shots), and
  dispatch.
- **`server.rs`** — the serve loop: `bind` the endpoint, then `accept` connections,
  rejecting any peer that is not `authorized()`. Each connection gets an id and an
  outbound mpsc channel drained by a writer task, so a streaming channel
  (`events.subscribe`) is an ordinary handler that pushes onto that channel. The
  loop runs under `tokio::select!` against a stop request (`daemon.stop`) and an OS
  signal (SIGTERM / Ctrl-C). Once listening, it spawns the tray helper
  (`tray.rs`) — best-effort, detached, skipped on a headless session or an
  endpoint override.
- **`runtime/`** — the daemon's long-lived collaborators in one place, the
  anti-churn seam a new subsystem hangs off (mirroring the engine's aggregate):
    - **`Runtime`** (`runtime/mod.rs`) — holds the `Engine`, the `EventHub`, the
      `JavaInstallManager`, the `DownloadManager`, and the `ProcessSupervisor`,
      plus the log path and a stop `Notify`. **`HandlerContext`** is what every
      handler receives: `{runtime, conn_id, out, peer}` — collaborators reached
      through `ctx.runtime.*()`, the outbound channel for streaming, and the
      verified peer (carried for a future auth check).
    - **`router.rs`** — `Router` maps a channel string to a handler; an unknown
      channel becomes a well-formed error response. `Channels` is the registrar:
      `on.handle::<C>(…)` decodes `C::Params` (a malformed payload answers
      `bad_request`), invokes the handler, and encodes `C::Result`, mapping a
      returned `ServiceError` (`not_found` / `bad_request` / `handler_error`) to its
      protocol code. The channel name and payload shapes come from the contract, so
      a handler physically cannot drift from the client SDK.
    - **`managers/`** — one module per manager: `DownloadManager`,
      `JavaInstallManager`, `ServerCreateManager`, `ServerUpdateManager`,
      `InstanceLaunchManager`, `BackupManager`, and `ContentManager`. The
      worker-thread pattern that lets `download.start` / `java.install` /
      `server.create` / `instance.launch` / `*.backup.create|restore` answer
      immediately while the blocking engine work runs off-thread, publishing
      progress/done/error events through the hub (the four backup job types
      share the `backup.progress|done|error` topics, disambiguated by job id).
      `managers/job.rs` is the plumbing they share: `topic_event`, the job-id
      generator, and `InFlight<K>` — the "one job per key" set whose `claim()`
      returns a guard that releases on drop, so a panicking job cannot wedge
      its key. The launch manager hands the prepared `LaunchPlan` to the
      supervisor under a deterministic process id (`server-<id>` /
      `instance-<id>`), so every channel can find a server's process without
      bookkeeping; the same id doubles as the backup in-flight key, which
      lifecycle handlers (start, update, remove) check so nothing swaps the
      tree an archive is reading.
    - **`scheduler.rs`** — the scheduled-backup loop: every minute, archive
      each *running* server whose `backup-interval` has elapsed since its
      newest backup (any kind — a fresh manual or pre-update archive resets
      the clock), then prune its `scheduled` archives beyond
      `backup-retention`. A stopped server's world cannot change, so it is
      never re-archived on schedule.
    - **`process/`** — `ProcessSupervisor`: launches processes whose lifetime
      is decoupled from the daemon's (own process group, no `kill_on_drop`, no
      pipes back to the daemon), tracks them, and applies a restart policy.
      Emits `process.started` / `process.output` / `process.exit`. Each live
      process has a record under `<data_home>/processes/<id>/` —
      `{pid, start-time token, spec}` (`records.rs`, owner-only: the spec can
      carry launch credentials) — and `recover()` re-adopts survivors at the
      next daemon start, verifying the pid against the start-time token
      (`identity.rs`, per-platform) so pid reuse is never mistaken for the old
      process. An adopted process is not our child: exit is detected by
      polling and its exit code reports `null`. Output lives on disk, not in
      pipes: `LogSource::File` points at a log the process writes itself
      (Minecraft's `logs/latest.log`; a `jvm.log` catches pre-log4j stderr),
      `LogSource::Capture` redirects into a supervisor-owned `output.log` —
      either way `tail.rs` polls the file for `process.output` events and
      `process.logs` reads its tail on demand, so log history survives daemon
      restarts. Stops are polite: SIGTERM (the JVM saves and exits), a hard
      kill only after a grace period. Cleanup is lifecycle-driven: a terminal
      state removes the record but keeps logs for post-mortem, removing the
      server/instance discards its process dir, and a startup sweep deletes
      recordless dirs.
    - **`event_hub.rs`** — `EventHub` fans daemon events out to subscribed
      connections, filtered by job id, and unsubscribes them on disconnect.
- **`services/`** — the single wire-in point, one registrar per domain
  (`lifecycle`, `config`, `cache`, `java`, `download`, `accounts`, `process`,
  `server`, `instance`, `backup`, `content`), each registering its channels with
  one `on.handle::<C>(…)` apiece; `services/mod.rs`'s `make_router()` is the list
  of `register()` calls, and `services/guards.rs` holds the preconditions the
  registrars share (`find_server`, `is_running`, `ensure_stopped`,
  `ensure_no_backup|update|content`, `require_backup`). Today: `health.ping`, `app.info`,
  `daemon.status|stop` (stop takes `stop_processes`; without it supervised
  processes keep running), `config.get|set|list` (the reserved `home`/`autostart` keys
  routed to the path pointer and login registration), `cache.info|list|clear`,
  `java.releases|list|install|uninstall`, `download.start`,
  `account.login.begin|login.complete`, `account.list|switch|remove` (`switch`
  picks the default account launches use; `list` reports it),
  `process.start|stop|list|status|logs`, `events.subscribe`,
  `server.flavors|versions|resolve`,
  `server.create|update|rename|list|status|remove|start|stop|logs|command`
  (create
  requires the caller to assert EULA acceptance; update refuses a running or
  still-creating server and, without `allow_downgrade`, a downgrade — a
  front-end updates a running server by explicitly stopping and restarting it
  around the job, the CLI's confirmed stop-update-start; rename re-slugs the id
  and moves the directory, refused while running or busy — see the decision
  note below;
  start/stop/status/logs are thin over
  the supervisor, merging the stored record with live process state; command
  relays one console command over the running server's rcon channel),
  `server.config.get|set|list` (the reserved `memory`/`jvm-args`/
  `backup-interval`/`backup-retention` keys on the record plus any
  `server.properties` key, bar the hestia-managed ports/rcon ones),
  `server.backup.create|list|restore|remove` (create archives a running
  server live; restore refuses a running or busy server and verifies the
  backup exists before answering with the job id), and the `instance.*`
  counterparts:
  `flavors|versions|resolve|create|update|rename|list|remove|worlds`
  (`worlds` lists a client's save worlds for the datapack picker), plus
  `instance.launch|stop|logs` (launch never refuses a running instance —
  each launch is a new session; `stop` fans out to every session or a named
  one; `logs` targets the newest running or a named session — all thin over
  the supervisor), `instance.backup.create|list|restore|remove` (create and
  restore require the instance stopped), and `instance.config.get|set|list`
  (`memory`/`jvm-args` only). Plus `sync.get|set` — the per-kind shared-config
  target sets (`get` returns both kinds; `set` takes a `kind` and validates each
  path: relative, no `..` escape, not a launcher-managed dir). Plus
  `content.sources|search|project|versions|modpack.resolve` — thin over the
  engine's content registry (an empty `source` selects the default; search,
  project, and versions are plain request/response, and `modpack.resolve`
  downloads the `.mrpack` index inline, so the client facade calls it with a
  longer timeout) — plus the per-entry install surface
  `server.content.add|list|remove|update` and its `instance.content.*`
  counterpart (add/update are jobs over a `ContentManager`, publishing the
  `content.*` topics; list/remove are plain request/response; all refuse a
  running or busy entry).
- **`autostart.rs`** — registers/removes the daemon as a login-time service per
  platform, driven by the `config` service when the reserved `autostart` key is
  set (`is_enabled()` / `set()`).

> **No Service-class-per-prefix — but one registrar function per domain.**
> Unlike the historical C++ tree (which had one `Service` *object* per
> channel-prefix, with its own lifetime and state), a handler here is a closure
> and the registry is a flat map from channel to closure. What a domain gets is
> only a `register(&mut Channels)` function: a compile-time grouping, no runtime
> entity. The grouping exists because the flat `make_router()` grew to ~75
> channels in one 1100-line function, which is the aggregation-point smell, not a
> design: wiring in a channel is still exactly one `handle::<C>` line, now in the
> file that owns its domain.

> **An aggregation point is a directory, not a file.** Four places in this
> codebase exist to gather every domain in one spot — the engine aggregate, the
> client's facades, the daemon's router, the daemon's job managers — and each grew
> linearly with the feature count until it was the largest file in its crate. The
> convention that caused it ("wire-in is one line, in one place") is right; the
> mistake was reading "one place" as "one file". Each is now a module directory
> where the aggregating seam stays thin (`make_router()` is a list of
> `register()`s; `Engine` is fields and getters) and every domain has its own
> file. Nothing about the crate graph, the wire, or the call sites changed —
> `Engine`'s flows are still `engine.provision_server(…)`, because Rust lets an
> inherent `impl` span modules within a crate. Splitting also surfaced the real
> duplication each file had been hiding: seven copies of a lock-insert-remove
> in-flight set became one `InFlight`/`Claim` guard, and four copies of a
> progress-decode closure became one `forward()`.

> **Workloads outlive the daemon by design.** The supervisor originally spawned
> children with `kill_on_drop` and piped output, which killed every server and
> game session on a graceful daemon stop — and leaked them untracked on a crash.
> Now the daemon is restartable/upgradable under live workloads (the same reason
> Docker grew `live-restore`): stopping a workload is always an explicit act
> (`server stop`, `process.stop`, `hestia daemon stop --all`), never a side
> effect of daemon lifetime. The cost is honest bookkeeping — on-disk records,
> start-time identity checks, file-based logs — and one observable gap: an
> adopted process's exit code is unknowable.

> **The server console is RCON, not stdin.** The input-side twin of the
> decision above: a stdin pipe exists only between a parent and the child it
> spawned, so it cannot be re-established for an adopted process (and dies
> with every daemon restart). RCON is re-establishable TCP state — any daemon
> can connect to any running server it knows the port and password for, which
> the server's record persists. Log streaming needed nothing new for the same
> reason: output already lives in files, tailed into `process.output` events.
> One caveat is inherited from vanilla: rcon has no bind-address setting, so
> the listener is network-reachable and the per-server random password is the
> only barrier (it never appears in logs).

> **An instance runs many sessions; a server runs one.** A client can be
> launched more than once at a time, so `instance-<id>` is no longer a single
> supervisor key — it splits into an *entry key* (`instance-<id>`, still the
> unit for the backup/update/content/rename guards and their in-flight sets) and
> a per-launch *session key* (`instance-<id>_<seq>`). Ids are slugs (`[a-z0-9-]`,
> never `_`), so a session prefix `instance-<id>_` can't collide across
> instances; every former singular lookup (status, stop, logs, running-check)
> becomes a prefix query over the supervisor's flat table, so the supervisor and
> its on-disk records need no change — each session just gets a distinct id.
> `stop` fans out to every session (or a named one); `logs` targets the newest
> running session (or a named one). Servers stay singular (`server-<id>`): a
> world has one authoritative writer. Two sessions of one instance share its
> single `data/` — Minecraft's own `session.lock` arbitrates a world, and each
> session gets a private log (below) so their output never interleaves.

> **Per-session logs come from a generated Log4j2 config, not a captured pipe.**
> Sessions share one `data/`, so they would all write `logs/latest.log`. Rather
> than capture each session's stdout (a pipe the daemon owns, which dies on a
> daemon restart and can't be re-established for an adopted process — the same
> constraint that made the console RCON), each launch is pointed at its own
> generated config via `-Dlog4j.configurationFile`, writing to
> `<instance>/logs/session-<seq>.log`. That is a real file the game writes, so
> it survives a daemon restart and the supervisor tails it by `LogSource::File`
> exactly as before. The generated config is Log4Shell-safe — `%m{nolookups}` in
> the pattern plus a belt-and-suspenders `-Dlog4j2.formatMsgNoLookups=true` — so
> overriding Mojang's bundled config never re-opens CVE-2021-44228 on the older
> versions Mojang had patched. The log lives under the instance root, not
> `data/`, so it stays out of backups.

## Front-ends: CLI, desktop, tray

### CLI (`cli`) — hestia

A thin client over the daemon, built on clap's derive API. `main.rs` defines a
`Command` enum — `play`, `account` (alias `auth`), `java`, `server`, `instance`,
the cross-entry shortcuts `start`/`stop`/`restart`/`logs`, `cache`, `config`,
`sync`, `daemon` — each a module under `commands/` exposing a `Subcommand` enum and a
`run()`. A domain with many verbs is a directory whose `mod.rs` holds only that
grammar and dispatch, with one file per verb group: `server/` and `instance/`
split into `create`, `update`, `backup`, `config`, `lifecycle` (plus the
server's `console`) over a shared `entry` module, and `content/` splits along
its own seam — `browse` (search a source) versus `manage` (install into an
entry). Global flags (`--verbose`/`--quiet`/`--home`) sit on the root; `--home`
is exported as `$HESTIA_HOME` and only takes effect when this invocation
auto-spawns the daemon (a running daemon keeps its own directory).
`commands/connect()` auto-spawns via the client SDK; `connect_running()`
requires an existing daemon.

The command grammar is noun-first and **entry-first**: catalogue verbs read
`hestia server create|list|versions|flavors`, but everything that acts on a
specific entry names it once, right after the noun —
`hestia server <name> <action>` (`server smp start`, `server smp config set
memory 4G`, `server smp backup create`). The name occupies one fixed slot
instead of floating to a different position per subcommand, which is what made
the old `server config smp set …` / `server backup create smp` mix
error-prone. clap models this with an `external_subcommand` variant on the
noun's `Subcommand` (`ServerCmd::Entry(Vec<String>)`): an unrecognised first
token — the entry name — is captured and re-parsed by a `no_binary_name`
`Parser` (`ServerEntry { name, action }`), so the per-entry actions keep full
clap help and validation while the catalogue verbs stay ordinary subcommands.
On top of that sit two deliberate cross-cutting shortcuts: `hestia play
[instance]`, the launcher's single most common action (picks interactively when
several instances exist); and verb-first `hestia start|stop|restart|logs|rename
<name>`, which resolve a name across *both* the server and instance registries
and dispatch to the right handler (a name that matches both asks the caller to
qualify it) — so day-to-day driving need not recall which kind an entry is, nor
that `server start` and `instance launch` differ. Anything a `create` needs but
wasn't given is asked for interactively (flavor/version pickers, the EULA
confirm) — on a terminal the picker *is* the browser; piped invocations error
with the flag to pass, so scripts stay explicit. `versions`/`flavors` (not
"available") name what they list, `ls`/`rm` alias every list/remove, and verbs
stay aligned with the wire channels (`remove`, not `delete`).

> **Entry-first, with verb-first shortcuts for the hot path.** The per-entry
> grammar used to be verb-then-entry, but the entry landed in a different
> argument position in every subcommand (`server start smp`, `server config
> smp set …`, `server backup create smp`), with no rule for where the name
> went — easy to get wrong and hard to remember. Fixing the name to one slot
> (`server <name> <action>`) removes that guesswork and lets each per-entry
> verb drop its own entry argument. The two exceptions to noun-first are
> earned, not sloppy: `play` and the `start`/`stop`/`restart`/`logs`/`rename`
> shortcuts
> are the actions taken often enough that making the user first pick the right
> noun (and remember `launch` ≠ `start`) is the friction worth paying a
> cross-registry name lookup to avoid. Everything scriptable still has an
> explicit, unambiguous noun-first form; the shortcuts are additive sugar over
> it.

**Presentation layer (`ui/`).** Commands **never print directly** — they build a
`View` (`Line`, `Note`, `Detail`, `Table`) and hand it to `ui::show`, which owns
all output. Interactive surfaces run as **fullscreen sessions** on a small
framework: `ui/session/` owns the terminal lifecycle (an RAII `TerminalGuard`
for raw mode + the alternate screen, released on drop — panics included), the
event loop (50 ms input poll, drain-before-redraw, dirty-flag drawing, resize
re-wrap), and the 80×24 minimum-size notice; a surface implements the
`Screen` trait (`draw`, `on_key`, `on_mouse`, `on_event`, `tick`) and composes
`ui/components/` (`TextInput`, `SelectList`, `LogView`, the searchable
`Picker`, the in-session progress gauge). Everything the CLI asks
interactively runs this way — the prompt screens behind `select`/`input`/
`confirm`, the searchable version picker, the table pager, the attach console,
the read-only log session, and the multi-step command flows (the content
browse→review→install session, the create wizards). A session with async
inputs (daemon events, search results) is fed through an mpsc channel by a
driver future the command runs alongside it (`Screen::Event`), and the outcome
prints plainly to stdout after the terminal is restored, so the scrollback
keeps a record. Piped or redirected, every surface degrades to plain text and
widgets degrade to arguments, so output stays scriptable. This is the seam for
the planned TUI: bare `hestia` (no subcommand) currently prints help, but the
intended end-state is a full-screen TUI over the same `Screen`s and `View`s
(à la the claude/codex model — a bare invocation is interactive, a subcommand
is scriptable).

> **Interaction is fullscreen; bare progress is one line.** The inline ratatui
> viewport (a fixed-height strip above the cursor) could not follow terminal
> resizes and left every widget fighting for rows, so it is gone: anything that
> takes keys owns the whole alternate screen for exactly as long as it runs,
> then hands the shell back intact. The deliberate exception is progress with
> no interaction (`java install`, `backup create`, a detached start): flashing
> the alternate screen for a spinner the user cannot act on is hostile, so the
> `Spinner`/reporter API renders one stderr line rewritten in place (and terse
> per-phase lines when redirected). Progress that happens *inside* a flow —
> installing a reviewed content batch, provisioning from the create wizard —
> renders in-session on the same screen that collected the decision.

> **One event-callback slot per client `Session`.** `run_job` and `subscribe`
> both claim the session's single event callback, so a session driver must
> serialize event-driven calls: plain request/response calls (search, detail,
> versions) may interleave freely, and the one job (`content.add`, the create,
> a log subscription) runs by itself. The content session and the wizards
> follow this rule; violating it silently drops events.

### Desktop (`desktop`) — hestia-desktop

A Tauri v2 shell hosting the React frontend in the root `frontend/`. **Today it is
the stock Tauri template** (a `greet` command in `lib.rs`) — the shell is
scaffolded but not yet wired to the daemon over the client SDK. The design rule,
once wired, is the same one-way boundary as the CLI: the shell owns windows and
IPC, and reaches launcher logic only through `client` (never by linking `engine`).
See [contributing.md](contributing.md) for the intended `#[tauri::command]` recipe.

### Tray (`tray`)

A resident system-tray helper, built on Tauri's own tray crates
([tray-icon](https://github.com/tauri-apps/tray-icon) + a
[tao](https://github.com/tauri-apps/tao) event loop; gtk/StatusNotifier on
Linux, native on Windows) and wearing the desktop app's icon (embedded from
`crates/desktop/icons/` at build time, so both front-ends share one face). The
menu is a status header (version + running/stopped), a start/restart action, a
start-at-login toggle bound to the reserved `autostart` config key, and a quit
that stops the daemon too (supervised workloads keep running, as with any
daemon stop). A worker thread polls the daemon every two seconds over the
client SDK and reports state changes to the event loop; menu actions travel the
other way over an mpsc channel, so the UI thread never blocks on the socket.
Left-click is deliberately inert for now — it will launch the desktop app once
the shell is wired to the daemon.

> **The daemon spawns the tray; the tray outlives the daemon.** `hestiad`
> spawns the tray on every serve (detached, like every workload), so the tray
> is simply always there when the daemon is — including a login autostart. It
> deliberately does *not* die with the daemon: a stopped daemon is exactly when
> the tray is most useful (the greyed status plus a start action), so only its
> own Quit removes it. The spawn is best-effort and unconditional — including
> under a `HESTIA_SOCK` override, since the spawned tray inherits the variable
> and follows its daemon's endpoint (the dev scripts run on a dev endpoint by
> design; a tray gated on the default endpoint would vanish exactly where the
> daemon is exercised most, and a hand-started one would control the wrong
> daemon). Only a headless session (no `DISPLAY`/`WAYLAND_DISPLAY` on Linux),
> a missing binary, or `HESTIA_NO_TRAY=1` (how the e2e test keeps its
> throwaway daemons quiet) means no tray. A duplicate spawn after a daemon restart is absorbed by
> the tray itself: it takes an exclusive lock keyed by its endpoint in the
> transport runtime dir (flock on POSIX, a no-sharing open on Windows) and
> exits at startup when another instance holds it — per-endpoint, so a dev
> daemon's tray and the session's tray coexist.

## What's built vs. pending

**Built end-to-end:** the workspace and its enforced dependency graph; logging,
identity, path resolution; the wire protocol and typed client SDK; the config
store; the download cache; Java runtime management (install/list/uninstall via
Adoptium); Microsoft account sign-in (device-code and sisu) with token rotation;
the daemon's process supervisor; the Minecraft provider layer (flavors, versions,
and profile resolution for vanilla and fabric, servers and instances); server
management (create = fully provisioned: profile + java + jar + EULA, each
server on its own claimed port; start/stop/status/logs over the supervisor;
a console over rcon — one-shot `command`, followed logs, interactive
`attach`); instance management (create a
record, launch materialises client/libraries/assets and spawns the game as the
signed-in account, and can run several concurrent sessions each with its own
Log4j2-routed log); shared settings/configs across servers and instances
(copy-based `sync`: `options.txt` merged, `config/` and others reconciled
newest-wins with a `shared/` store at each start/launch); in-place version
updates for both (downgrades gated
behind an explicit confirmation, the existing data backed up automatically
first); backups for both (on-demand archive/restore with live progress — a
running server is archived under the RCON save-off dance — plus per-server
scheduled backups with retention pruning); the content provider layer
(Modrinth search/project/versions/modpack resolution) with per-entry
install/management — mods and datapacks on servers, mods/resourcepacks/shaders/
datapacks on instances, from a platform project, a source page URL, or a local
file, with required dependencies resolved and a `data/` mirror that heals across
backups (datapacks install into their world, which the world backup already
covers); the kind-first browse and management CLI (`hestia mod search`,
`instance <name> mod add|list|remove|update`, `hestia search`); the CLI
front-end over all of it; and the system tray (spawned by every serving
daemon, quick actions for start/restart/autostart/quit).

**Pending:** natives-classifier extraction for pre-1.19 clients (the resolver
skips legacy `natives-<os>` classifier libraries, so old versions launch
without their LWJGL natives) and the legacy (virtual) asset layout; installing
a resolved modpack (its files and `overrides/`, e.g. `instance create
--modpack`); wiring the desktop shell to the daemon; and the tray's
left-click launching the desktop app.

> **Server provisioning is front-loaded by design.** A server is a long-lived,
> repeatedly-started thing, often driven headless/scripted — `create` pays the
> whole cost once (jar, java, EULA) so `start` is an immediate spawn that cannot
> fail on the network. An instance is the opposite: cheap to create, and its
> heavyweight files (client jar, shared libraries, thousands of assets) are
> ensured idempotently at launch, shared across instances via the
> `meta/libraries/` / `meta/assets/` / `meta/versions/` roots.

> **Materialised game files live under one `meta/` root.** The data home holds
> what a user would recognise as theirs (`instances/`, `servers/`,
> `accounts.json`, `config.json`), the launcher's internals (`cache/`, `logs/`,
> `processes/`), and the `java/` runtimes; the game files the launcher
> materialises at launch — `versions/`, `libraries/`, `assets/`, `natives/` —
> sit under `meta/`. This is the Modrinth (Theseus) layout; Prism-style
> root-level sprawl buries the user's own directories among derived,
> re-downloadable ones. `meta/` is also one obvious unit to reclaim:
> everything under it is regenerated on demand. Natives are per-version
> (`meta/natives/<version>`), not per-instance, so the instance directory
> stays a pure game dir.

## Tests

- `crates/proto/tests/` — `wire` and `golden`: the envelope and contract encodings
  are pinned so a wire change is caught.
- `crates/engine/tests/` — `store` (config/cache/java/servers/instances
  persistence) and `auth_oracle` (the account sign-in state machine); launch-plan
  assembly (classpath, placeholder substitution, the per-session log-config
  injection) is unit-tested in `minecraft/launch.rs`, the Log4Shell-safe
  session config in `minecraft/log4j.rs`, the copy-based config reconciliation
  and `options.txt` merge in `sync.rs`, the Modrinth mapping and `.mrpack`/URL
  parsing in `content/modrinth.rs`, and content version-pick / reference-matching
  in `content/install.rs`.
- `crates/daemon/tests/e2e.rs` — a client-to-daemon round trip over a real
  socket; the session-key prefix invariant is unit-tested in `runtime/mod.rs`.

Run the fast core with `cargo build -p cli -p daemon`, then
`cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.

## Recording a decision

When a non-trivial architectural choice is made, capture *what* changed and *why*
here, next to the structure it explains, so this file stays the single source of
truth rather than letting the reasoning drift into commit messages or chat logs.
