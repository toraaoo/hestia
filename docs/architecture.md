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
| Desktop   | `hestia-desktop` | `desktop` | Tauri v2 + React/Vite | daemon-wired; UI pending   |
| Tray      | `tray`           | `tray`    | tray-icon + tao       | shipped                    |

The daemon (`hestiad`) is the resident core. The CLI is the first-class,
fully-wired front-end; the tray accompanies every serving daemon with quick
actions; the desktop is wired to the daemon but its UI is not built yet (see
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
engine  → config·cache·download·java·accounts·skins·minecraft·content·process → proto, common  (daemon-only)
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
- [fastnbt](https://github.com/owengage/fastnbt) — reading a save world's own
  `level.dat` (gzipped NBT) through serde.
- `sha1`/`sha2`, `tar`+`flate2`, `zip` — in-process checksums and archive
  extraction (no shelling out to system tools).
- [Tauri v2](https://tauri.app/) + [React](https://react.dev/)/[Vite](https://vitejs.dev/)
  (built with [Bun](https://bun.sh/)) — desktop; its
  [tray-icon](https://github.com/tauri-apps/tray-icon) +
  [tao](https://github.com/tauri-apps/tao) crates — the system tray.
- [three.js](https://threejs.org/) — the desktop's skin preview and its card
  thumbnails (see the skin-preview decision note).

## The socket boundary

Every request crosses the same seam. Two crates own it.

### `proto` — the no-drift wire contract

`proto` is pure data: no I/O, no async, `serde` derive is the codec. Both sides of
the socket marshal through **one** definition per channel, so the daemon and every
client cannot disagree — a mismatch is a compile error, not a runtime surprise.

**The wire is camelCase.** Every fielded proto struct carries
`#[serde(rename_all = "camelCase")]` (Rust field names stay `snake_case`; only the
serialized form is renamed), so the socket speaks camelCase to the CLI and the
desktop alike — the webview consumes it directly, with no key conversion. Enum
variant *values* stay `snake_case`/`lowercase` (the front-end's string-literal
types depend on them), so enums keep their own `rename_all`. `tests/casing.rs`
fails the build if a new serialized struct omits the attribute. The single
deliberate exception is the `config.*` key vocabulary, which stays kebab-case
(see the config decision note below).

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
`download`, `java`, `accounts`, `skins`, `process`, `server`, `instance`,
`events` —
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
  `UNKNOWN_CHANNEL`, `HANDLER_ERROR`, `VERSION_MISMATCH`, …) and the client-facing
  `IpcError`.

> **The envelope seam fails closed, so no decode site can forget the version
> check.** `PROTOCOL_VERSION` is `1`, same-major only — but the rule is only
> real if every decode enforces it. `compatible()` began as a free helper with
> *zero* call sites: `decode_request` handed the version out as a plain field
> each caller was free to ignore, and defaulted a **missing** `v` to the current
> version, so any envelope (`v: 0`, `v: 99`, absent) decoded into a valid request
> and dispatched — the seam failed *open*, and the "same-major only" rule was
> unenforced anywhere in the tree. Now the type carries the invariant:
> `decode_request`/`decode_response` return a typed `DecodeError`
> (`Malformed` vs `IncompatibleVersion { got, want }`) and **refuse to construct
> a frame at all** for a foreign major or a missing `v` (a missing version is
> malformed, never an implicit "current"). The daemon maps that error to a
> `version_mismatch` response (`ErrorInfo::IncompatibleVersion`) and the client
> tears the connection down rather than silently consuming a foreign-major
> daemon — a junk frame is still ignored, but a version mismatch is refused. Both
> directions are pinned in `protocol.rs`'s unit tests. The rejected alternative
> was an `if !compatible(v)` guard inside the daemon's serve loop: a band-aid
> that leaves the same hole open at every other decode site and keeps the check
> opt-in for the next person who decodes a frame.

## `common` — cross-cutting

UI-free, domain-free code linked by the daemon and every client:

- **`app`** — the application identity constants (`NAME`, `ID`, `VENDOR`,
  `CHANNEL`, `VERSION` from `CARGO_PKG_VERSION`): one source of truth every binary
  reads.
- **`logging`** — `init_logging(console LogLevel, Option<FileLog>)` configures the
  process `tracing` subscriber once, installs the crash hook, and returns a
  `LogGuard`. Each binary owns a directory under `<data_home>/logs/<binary>/` and
  writes up to three independently filtered sinks: the console (stderr, gated
  while a fullscreen CLI surface owns the terminal), `latest.log` (Hestia's own
  crates at the file's level, dependencies at warnings), and — for the resident
  binaries, `hestiad` and `desktop` — the `debug/latest.log` firehose (every
  target at trace, dependencies included). `HESTIA_LOG` overrides every computed
  directive. Rotation is `flexi_logger`'s: `latest.log` rolls on the day or 20 MB
  into dated `YYYY-MM-DD.log.gz` archives (2 kept plain, 30 gzipped), the
  firehose on 200 MB (5 gzipped) — Minecraft's own `logs/` layout and Forge's
  split.
- **`crash`** — the panic hook every binary gets from `init_logging`: a panic is
  recorded through `tracing` *and* written to `<data_home>/logs/crashes/` as a
  standalone report (message, location, backtrace, platform, and the tail of the
  live log). `record()` is the same path for a crash that never touched the Rust
  stack — the desktop's webview errors. `list`/`read`/`clear` back the shell's
  crash notice; `read` only opens paths the module itself wrote.
- **`time`** — local-time stamps for log lines and report names, via `chrono`.
- **`paths`** — data-directory resolution: `--home` → `$HESTIA_HOME` → a persisted
  pointer (`config set home`) → the platform default (`~/.hestia`, `%APPDATA%\Hestia`
  on Windows). **Debug builds** anchor the default at `<workspace>/.hestia` so
  development never touches the real per-user directory. Also `config_path`,
  `log_dir`, and `set_persisted_home`.

> **Two log files, because one file cannot be both readable and complete.** A
> single sink forced a choice every debug session lost: at trace it filled with
> dependency chatter (`latest.log` was 90% `mio::poll` plus a `daemon.status`
> poll every two seconds), and below trace it dropped the detail a bug needed.
> The split is Forge's — a filtered `latest.log` for reading and a `debug.log`
> firehose for reconstructing — and it is only possible per *target*, so the
> sinks carry their own `EnvFilter` as per-layer filters rather than sharing one
> global one. The firehose is deliberately unfiltered, inheriting the global
> filter instead: it must take everything, dependencies included. Rotation moved
> to `flexi_logger` rather than staying hand-rolled, which retired four bugs in
> the old `rolling.rs` — a failed rotation re-gzipped the whole file on every
> subsequent write, a failed archive then truncated the log it had failed to
> save, archives were written non-atomically, and pruning sorted by mtime so an
> unreadable archive was deleted first. Only the writer is borrowed: the
> subscriber, formatting and spans stay `tracing`'s, because `flexi_logger`'s own
> `trc::setup_tracing` installs a single-writer subscriber that cannot express
> three differently-filtered sinks.

> **A crash must survive the process that had no console.** The daemon's stderr
> is detached, a release desktop build has `windows_subsystem = "windows"`, and a
> panic inside a spawned task printed where nobody looks — so a crash left
> nothing but a missing process. `init_logging` now installs the hook for every
> binary (rather than each `main` remembering to), and reports from all four
> share one directory so the desktop can surface a *daemon* crash it never saw.
> The webview reaches the same reports through `crash_report`, since a React
> render error or an unhandled rejection kills the UI without touching the Rust
> stack. Note that `panic = "abort"` with `strip = true` in the release profile
> leaves release backtraces as bare addresses.

## `client` — the typed SDK

The one way a front-end drives the daemon. `Client::connect()` opens a connection
to a running daemon — it never spawns; `Client::start()` is the sole path that
spawns `hestiad` (if not already running), backing the deliberate start actions
(CLI `daemon start`, the tray, the desktop start button). `connect_to(endpoint)`
targets an explicit socket.

- **`Session`** (`session.rs`) — the connection core, private to the crate: one
  persistent, multiplexed connection whose background reader task fulfils pending
  requests by id and delivers events to an installed callback. `call::<C>()`
  marshals through the contract and returns the `proto` result directly;
  `try_call` maps a `not_found` to `None`; `call_with_timeout` overrides the 10 s
  default; `run_job` drives a long-running operation, forwarding its progress
  events and blocking until a done/error topic arrives. It is also where the
  **wire tracing** lives (`trace!` per frame sent/received, plus connection
  transitions) — the CLI's `-vv` — see the decision note below.
- **facades** (`facades/`) — one struct per domain in its own module, reached
  through a `Client` accessor (`client.java().install(21, …)`), mirroring the
  engine's domain modules on the other side of the socket. Facade methods are
  one-liners over `Session`: `App`, `Daemon`, `Config`, `Cache`, `Java`,
  `Accounts`, `Skins`, `Process`, `Server`, `Instance`, `Content`. `facades/jobs.rs` holds
  the drivers the server and instance facades share — the backup (server-only)
  and content jobs publish the same topics, disambiguated by job id.
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
  unknown keys and type-mismatched values — the struct *is* the validation,
  plus a `normalize()` pass for value rules the type can't carry (the
  `defaults.memory` / `defaults.jvm-args` keys reuse the per-entry
  `memory`/`jvm-args` validation). Those JVM defaults fall back into any
  server start or instance launch whose record leaves the matching per-entry
  setting unset (`JavaSettings::or_defaults`). The reserved `home` and
  `autostart` keys are routed by the daemon to the path pointer and the login
  registration rather than the store. `reload()` repoints it on a data-home
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
- **`skins`** (`Skins`) — the skin library: PNG textures the user saved, kept
  under `<data_home>/skins/` as `<key>.png` blobs beside a `library.json` index
  (the disk is the registry, as with `java`). A row is keyed by Mojang's texture
  hash — an upload response reports the minted key and the row follows it — so
  matching the account's equipped skin at list time is a key comparison.
  `skins/mojang.rs` holds the profile-customization HTTP calls (profile fetch,
  multipart skin upload, by-URL skin change, reset, cape set/clear) against
  `api.minecraftservices.com`, bearer-authed with the accounts subsystem's
  rotated token; a 30 s per-account profile cache absorbs bursts of
  `skin.list` reads (Mojang rate-limits hard) — a change stores the profile
  its response carries, or drops the entry so the next read refetches; `skins/defaults.rs` is the table of the eighteen vanilla
  default skins (nine characters × two model variants) — nothing bundled, since
  Mojang serves every texture publicly by its hash. The flows composing
  accounts + library (`engine/flows/skins.rs`) build the merged picker list and
  run the changes — see the decision note below.
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
  registries. A *flavor* is a distribution (`vanilla`, `fabric`, and the
  server-only `paper`, `folia`, `spigot` and `bukkit`, plus `neoforge`); a
  provider names itself (`id`, `name`, `summary`), lists
  the game *versions* it supports, states what content its loader takes
  (`loads`), and *resolves* a request into a launch profile —
  the full descriptor (`ServerProfile` / `InstanceProfile`: primary artifact,
  libraries, asset index, java major, main class, args) the launch pipeline
  consumes. The two registries are separate, so a flavor can serve one side
  only: Paper, Folia, Spigot and CraftBukkit have no client and appear in
  `server.flavors` alone.
  Stateless (every result is fetched upstream), so it needs no data
  directory. Manifest parsing lives in `minecraft/meta/` (`mojang`, `fabric`,
  `paper`, `spigot`).
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
  A server takes whatever its flavor loads — mods on fabric, plugins on
  paper/folia — plus datapacks; instances take mods, resourcepacks, shaders,
  and datapacks. `Engine` composes the flows
  (`add_server_content`/`add_instance_content`, list/remove/update) and a
  `sync` pass re-mirrors any missing managed file at every start/launch (below).
- **`modpack`** (`content/mrpack.rs`, `content/modpack.rs`) — installing a whole
  pack. `mrpack` owns the *format* (the manifest, and extracting the
  `overrides/` trees), deliberately apart from the platform that serves it: a
  pack picked off disk has no source and is read the same way. `modpack` is the
  per-entry record (`<entry>/modpack.json`) of which pack an entry runs and
  which game-directory files it owns, with the hash each was written with.
  `Engine`'s flows (`engine/flows/modpack/`) compose them: `resolve` turns a
  reference — a project, a source page URL, or a local `.mrpack` — into an
  archive, and `apply` puts it onto a new or existing entry. See the decision
  note below.
- **`sync`** (`Sync`) — shared settings/configs propagated across instances
  through a persistent `<data_home>/shared/` store (one flat store, one
  `targets.json`). Two target classes: **files are copied** (`options.txt`
  key-merged, `servers.dat` newest-wins) and **folders are linked**
  (`saves`, `config`, `screenshots` — a symlink on POSIX, a junction on
  Windows, via `sync/link.rs`), so folder content is stored once and shared
  live. `apply` runs at every launch (hooked into `prepare_instance`,
  before the content re-mirror) and once at create, before anything can fill
  a folder: it reconciles the file targets and links each folder target,
  **adopting** one that already holds the instance's own files (a name the
  store already has is the only thing that stops the move); `status` reports
  each instance's per-target link state and `adopt` is the same migration on
  demand. A `Settings` scope says where the settings-class targets reconcile
  — the global store, a captured profile's, or nowhere (a modpack owns its
  config tree). Sharing is switchable wholesale (`sync.enabled`). Sync is
  **instance-only**: servers are deliberately decoupled from it. The managed
  content dirs are rejected as targets at the edge — see the decision note
  below.
- **`process`** (`ProcessSupervisor`) — launched processes whose lifetime is
  decoupled from the daemon's (own process group / job object, no
  `kill_on_drop`, no pipes back), tracked with a restart policy. Each live
  process has a record under `<data_home>/processes/<id>/` —
  `{pid, start-time token, spec}` (`records.rs`, owner-only: the spec can carry
  launch credentials) — and `recover()` re-adopts survivors at the next daemon
  start, verifying the pid against the start-time token (`identity.rs`,
  per-platform) so pid reuse is never mistaken for the old process. An adopted
  process is not our child: exit is detected by polling and its exit code
  reports `null`. Output lives on disk, not in pipes: `LogSource::File` points
  at a log the process writes itself (Minecraft's `logs/latest.log`; a
  `jvm.log` catches pre-log4j stderr), `LogSource::Capture` redirects into a
  supervisor-owned `output.log` — either way `tail.rs` polls the file for
  `process.output` events and `process.logs` reads its tail on demand, so log
  history survives daemon restarts. Stops are polite: SIGTERM (the JVM saves and
  exits), a hard kill only after a grace period — both addressed at the whole
  tree. Cleanup is lifecycle-driven: a terminal state replaces the record with a
  **tombstone** (`exit.json`) so the directory keeps its logs *and* says what it
  is, removing the server/instance discards its process dir, a startup sweep
  deletes only directories with neither marker, and retention prunes the oldest
  tombstoned dirs. `task.rs` is the provisioning half: `run(Task, Job)` drives a
  program to completion, relays what it narrates as progress and is cancelled
  through the supervisor, so no engine flow spawns a bare child.
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
  server's own derived `schema.properties` carries, so a typo cannot silently
  drift the file; the hestia-managed ports/rcon keys are rejected — see the
  decision note below). An entry directory holds the record beside `data/`, the
  game's own working directory; the root is reserved for the managed content
  directories (`mods/` and `backups/` for servers; `mods/`, `resourcepacks/`,
  `shaderpacks/` for instances) and the `content.json` install index, each
  created on demand — see the decision note below:

  ```
  servers/<id>/               instances/<id>/
  ├── server.json             ├── instance.json
  ├── schema.properties       ├── content.json
  ├── content.json            ├── mods/ resourcepacks/
  ├── mods/ backups/          │   shaderpacks/
  │                           │
  └── data/                   └── data/
      jar, libraries/,            saves, options, logs,
      eula.txt,                   mods/ (mirror) —
      server.properties,          the game dir the client
      world, logs, mods/          writes into
  ```

- **`backup`** — server backups: gzipped tar archives of a server's `data/`
  under its `backups/`, named `<utc-stamp>-<kind>.tar.gz` (kind = `manual` /
  `scheduled` / `update`) — the disk is the registry, here too. Creation
  skips what the launcher re-materialises (the server jar, `libraries/`,
  `logs/`, `cache/` — docker-mc-backup's default exclude set — plus the managed
  content mirror `mods/` and transient `session.lock` files) and writes through
  a `.part` temp file; restore
  extracts into a
  staging directory, carries the skipped top-level names over from the current tree
  (they belong to the record's *current* version), and swaps — a failure
  leaves the current data untouched. `prune` keeps the newest N of one kind.
  Every pass reports per-file progress. Backups are a **server** feature:
  instances have none — import/export is the intended replacement and is not
  built yet, so instance data currently has no backup story at all.

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
  version and swap the record's profile — a server takes an automatic
  `update`-kind backup of its existing data first and re-materialises its
  files under the `ready` gate, regenerating its properties schema; an
  instance pays at the next launch, and **nothing of it is backed up** —
  its downgrade warning says so). Both directions work; a downgrade must be
  allowed explicitly, and the direction is judged by position in the flavor's
  own newest-first catalogue, not by parsing version strings. The aggregate
  also composes the backup flows over the `backup` module: `backup_server` (a
  live server's world saving pauses over RCON around the archive — `save-off`,
  `save-all flush`, tar, `save-on`, with `save-on` retried even when
  archiving fails, exactly docker-mc-backup's sequence),
  `restore_server_backup`, and `prune_server_backups`; one backup *or*
  restore runs per server at a time.
  Servers are fully provisioned at create so `start` is an immediate spawn;
  instances are records at create and pay at launch.

> **NeoForge's game jar is built, not downloaded — so a flavor can install.**
> NeoForge publishes no metadata service and no patched jar. Everything comes
> out of its installer jar on `maven.neoforged.net`, read in-process with the
> `zip` crate as a `.mrpack` index is: `version.json` is the launch profile,
> `install_profile.json` names a chain of ten small Java tools, and running that
> chain locally produces `net.neoforged:neoforge:<v>:{client,server}` — the jar
> the loader actually runs. This follows theseus's technique but reads the
> installer directly rather than Modrinth's pre-processed copy, keeping the
> upstream-direct rule every other flavor follows and taking libraries from
> NeoForged's own maven with their own checksums. Two things follow from that
> choice, both normalised away before theseus sees them: the data table's
> `/data/*.lzma` binary patches are extracted from the installer, and
> substitution is side-aware (theseus is client-only and reads each entry's
> `client` value).
>
> Building a jar is not something a profile can express, so the providers grew
> an **`install` hook** rather than the launch flows branching on a flavor name.
> It is idempotent on the patched jar's presence — the chain is minutes of JVM
> work — and each processor is a cancellation checkpoint, leaving exactly what a
> failed processor would.
>
> The catalogue needs no service either: a NeoForge version *is* its game
> version plus a build number, under two schemes split by Minecraft's move to
> calendar versioning (`21.1.244` → 1.21.1; `26.2.0.35-beta` → 26.2, a zero
> patch or hotfix dropping). The rule reproduces Modrinth's published manifest
> exactly across all 1629 published versions — artifacts included, since an
> April Fools' build maps to a version that does not exist. Filtering the result
> against Mojang's manifest drops it: a mapping naming no real version is a
> failed derivation, not a version to offer.
>
> A **server** has no launchable jar at all. Its install generates an argument
> file naming the module path, system properties and launch target — far past
> what a command line carries — so `ServerProfile` gained `args_file` and the
> server runs as `java @libraries/net/neoforged/neoforge/<v>/unix_args.txt
> nogui` (`win_args.txt` on Windows). The path stays *relative* because the file
> names its own libraries that way and is only valid from the data directory.
> That reordered provisioning: `server.properties` is derived by running the
> server once, which a flavor that builds its server cannot do until the install
> has finished, and the install needs the jar provisioning fetched. The three
> are now ordered by the flow — fetch, install, derive — rather than nested, and
> `update` follows the same order. The vanilla server jar stays the profile's
> primary artifact even though it is never launched: it is the input the
> processors patch, and keeping it there is what makes provisioning fetch it.
>
> **The schema run therefore ignores the argument file.** A NeoForge server's
> property schema used to be underivable — twice over: the generated argument
> file resolves its libraries relative to the *data* directory, so it cannot run
> from the throwaway dir, and FML gates on the EULA *before* vanilla writes
> `server.properties` (vanilla writes it first), so even running it there yields
> no file. Every NeoForge create then reported `PropertiesSchemaMissing`, a
> warning about nothing the user did and nothing they could fix — its own hint
> pointed at `update`, which failed identically.
>
> The fix is what the profile already carries: the vanilla server jar. A
> properties schema *is* the vanilla key set for that game version — the loader
> contributes none, and no mods are installed at create — so
> `server_schema_plan` drops `args_file` and boots the primary artifact
> (`-jar server.jar nogui`), which stops at the EULA gate having written the
> file, exactly as every other flavor does. Nothing else in the pipeline
> changed, and the warning now fires only for a run that genuinely failed (a
> timeout, a crash), which is a thing worth saying.

> **A Paper build is a loader version, and Mojang orders the catalogue.**
> Paper and Folia are one self-contained jar per build, so a profile is the
> vanilla shape with the jar swapped — the interesting parts are what the
> PaperMC API does *not* say. It publishes many builds per game version, and a
> server operator routinely needs a specific one (pinning a known-good build,
> or taking an experimental one deliberately), which is exactly what
> `loader_version` already means for Fabric's loader builds — so a build number
> goes there rather than growing a parallel concept. Unpinned resolves to the
> newest `STABLE` build, falling back to the newest of any channel: a freshly
> released game version whose builds are all experimental would otherwise be
> uninstallable until PaperMC promoted one.
>
> Ordering and stability come from **Mojang's manifest**, not PaperMC's. Fill
> groups versions under a JSON object keyed by version group, and a parsed
> object sorts its keys as strings — which puts `1.9` after `1.21` and would
> silently invert `downgrade_between`, the one place ordering is load-bearing.
> The manifest is already the ordering ground truth every other flavor is
> judged against, and it carries release/snapshot besides, which PaperMC never
> states. A version Mojang does not list keeps its place at the end of the list
> as a snapshot, so an April Fools' build is still creatable rather than
> vanishing from the catalogue.
>
> The API itself moved: Fill v2 (`api.papermc.io`) stopped receiving builds at
> the end of 2025 and was disabled on 1 July 2026, and v3 refuses a request
> whose user agent does not identify its caller — so `common::app::user_agent`
> now builds one identity for every outbound request rather than paper alone.

> **A flavor states what it needs, before the user commits to it.** Spigot and
> CraftBukkit are the first flavors that can be *unavailable on this machine*:
> BuildTools drives git, and bootstraps its own only on Windows. Failing at
> create would tell a user who has never heard of git that something went wrong
> minutes in, so `Flavor` carries `requires` — the prerequisites resolved as
> **missing** when the catalogue is built, each with a name the user would
> recognise and where to get it. A front-end renders them beside the flavor
> without knowing which flavor needs what; the refusal itself is
> `ErrorInfo::MissingRequirement`, the same structured shape. The check is
> `Engine`'s, not `Minecraft`'s: whether a tool is installed is a question about
> this computer, and the catalogue stays a pure read of the providers.

> **Spigot and CraftBukkit are compiled here, because no one may ship them.**
> Mojang's takedown means neither jar legally exists as a download: SpigotMC
> publishes **BuildTools**, which clones the four upstream repositories,
> decompiles the vanilla server, applies the CraftBukkit and Spigot patch sets
> and compiles the result on the user's own machine. So these flavors take the
> `install` hook NeoForge already established for a jar that has to be built,
> and the same shape holds — the profile *names* the jar the launch plan runs
> (`spigot-<version>.jar`) while carrying no URL, which is exactly what tells
> `Servers::provision` there is nothing to fetch. Deliberately no third-party
> mirror fallback: the mirrors redistribute what the takedown covers, publish
> no checksums, and would silently change the trust story mid-create.
>
> **One build serves both flavors and every entry on that version.** A
> BuildTools run is minutes of decompilation and maven over a few hundred
> megabytes of clones, and it emits `craftbukkit-<v>.jar` *and*
> `spigot-<v>.jar` from the same work — so building per-server would pay that
> cost again for a jar already on disk. The work tree is therefore shared
> (`meta/spigot/`, with the outputs under `jars/<version>/`), which is why
> `InstallRequest` grew a `meta` root beside the entry-scoped `root`: an
> instance install already writes to `meta/`, and a server's did not have it.
> The jar is copied from there into the entry's `data/`, so a server still owns
> its own copy and the existing backup exclude/carry-over rule (keyed on
> `primary.filename`) needs no change.
>
> **The catalogue is a filter, not a listing.** The hub indexes its version
> metadata by Jenkins build number as well as by game version, so all but a few
> dozen of the four thousand names it publishes are build numbers. Filtering
> against Mojang's manifest is what leaves the game versions behind — the
> inverse of Paper, where an unlisted name is kept as a snapshot. There is one
> build per game version rather than a stream of them, so neither flavor has a
> loader version to pin. The Java major comes from the hub's class-file range,
> narrowed to a runtime the launcher can actually install so a mismatch fails
> at resolution rather than at the Java step.
>
> **The build is a supervised workload, not a bare child.** It drives git,
> maven and a decompiler JVM, so it runs through `ProcessSupervisor::run` like
> a server does: its output is captured to a file (`hestia process logs
> build-spigot-<version>` reads it live), a cancel reaches the whole tree, and a
> build that outlives a daemon restart is re-adopted rather than orphaned. Its
> id is derived from the game version, so two creates racing on one version —
> or a create after a restart — join the build already running instead of
> starting a second.

> **What an entry takes is a property of its flavor, and the flavor says so.**
> Two guards used to hard-code the answer: one refused anything but mods and
> datapacks on a server, the other refused mods on vanilla *by name*. Paper
> breaks both — it loads plugins, which are neither. The rule is now composed
> from two independent facts: what the **flavor's** loader consumes (a
> `ContentKind` on `ServerProvider`/`InstanceProvider` — mods for a modloader,
> plugins for a server platform, nothing for vanilla) and what the **side**
> reads for itself (a client its resourcepacks and shaders; either side the
> datapacks that are world data rather than loader content). Adding a flavor
> stays one impl plus one registry line, with no edit to the content flows —
> which is what the old tables cost every time.
>
> A refusal carries the accepted set (`ContentKindRejected`) instead of a
> sentence, because a sentence goes stale the moment a flavor is added; the two
> `Unsupported` variants that spelled it out are gone. And the *front-end* gets
> the set on the wire — `ServerInfo`/`InstanceInfo` carry `accepts` — rather
> than keeping its own copy. It had one (`ACCEPTS` per entry type plus a
> `flavor === 'fabric'` test), it was already wrong for neoforge, and that is
> precisely the drift the no-drift seam exists to prevent.
>
> **A flavor therefore describes itself on the wire, catalogue included.**
> `Flavor` is not `{id, name}`: it carries the `summary` a picker renders and
> the `accepts` set an entry of it *would* have, so shipping a flavor is a
> daemon-side change alone. The composition itself (`accepted_kinds`) sits
> beside the provider trait that defines `Loads`, so the catalogue and an
> existing entry's `accepts` cannot disagree. The front-ends were each keeping
> the missing half: the CLI's flavor table was `ID`/`NAME` and its picker a bare
> name, and the desktop looked up a per-flavor `flavor.<id>_summary` message
> that a new flavor simply did not have (rendering blank). Both now read the
> wire — the desktop still *prefers* its own translation when it has one and
> falls back to the daemon's English, so a new flavor renders in every locale
> immediately and a translated one stays translated.
>
> Plugins otherwise reuse the managed-dir model unchanged: `<entry>/plugins/`
> mirrored into `data/plugins/`, provenance in `content.json`, `plugins/` added
> to the backup exclude set so a restore heals it. **Folia is filtered strictly
> as `folia`**, never widened to `paper`: a plugin that never claimed Folia
> support breaks on its regionised scheduler, and the catalogue is the only
> place that is knowable. Verified against a paper-only plugin, which installs
> on Paper and is refused on Folia at the same game version.

> **A flavor may recommend JVM flags; the user still outranks it.** PaperMC
> publishes a tuned G1GC set per version, and running a Paper server without it
> is measurably worse — so `ServerProfile` carries `jvm_args` and they become
> the last fallback beneath the entry's own `jvm-args` and the launcher-wide
> `defaults.jvm-args`. No new mechanism was needed: `or_defaults` already fills
> only what a layer left unset, so the flavor chains onto the existing call.
> Memory is deliberately excluded — how much RAM to give a server is the user's
> call, not a catalogue's.
>
> The cost is flags the user never typed, which `config get jvm-args` would
> honestly report as unset while the server ran with eighteen of them. That is
> the hidden-behaviour failure this codebase already rejects elsewhere, so
> `server info` names the effective flags **and which layer supplied them**
> (`JvmArgsSource`). A front-end must not have to guess why a process has flags
> nobody set.

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
> in-flight keys) — that materialize step landed later (below), and the wire
> contract did not change when it did.

> **A modpack is three things at once, and each goes where it already belongs.**
> Installing a pack could have been a store of its own — a `modpack/` tree, its
> own mirror, its own update logic. It is not, because a pack decomposes cleanly
> into things this codebase already has:
>
> - its **loader and game version** become the entry's flavor and version, so
>   creating from a pack is the ordinary create with those filled in (which is
>   also why `instance|server create --modpack` and `modpack install` are one
>   code path);
> - its **index files under a flat managed load dir** become ordinary pool items
>   tagged `modpack:<project>`, so the launch-time mirror, the backup heal,
>   `content list`, per-item enable and per-item update all work on them
>   unchanged — the same origin-tag mechanism global profiles already use;
> - **everything else it ships** — its `overrides/`, plus any index file outside
>   a managed dir — is written straight into `data/` and recorded in
>   `modpack.json`.
>
> Only the third needed anything new, and only for one reason: those files are
> configs and keymaps *the user edits*. So the record stores the sha1 each was
> written with, and an update replaces a file the pack still owns while leaving
> a tweaked one exactly as found, reporting which through
> `WarningInfo::ModpackOverridesKept`. They are not given a managed copy of
> their own: unlike a jar, a config lives inside the backup archive already, so
> a restore covers them and a second copy would double every config on disk to
> re-solve a solved problem. Both references agree on this much (Modrinth's
> launcher and Prism both extract overrides in place and track their hashes);
> the divergence is that hestia's *mods* are pool items rather than pack-owned
> files, which is what makes a pack's mod individually updatable.
>
> **The server side is new ground.** Both references are client-only — Modrinth
> handles `overrides/` and `client-overrides/` and skips `server-overrides/`
> outright — so the server half follows the format spec rather than a
> precedent: `env.server` decides which index files are wanted, and
> `server-overrides/` takes the place of `client-overrides/`. The shared tree is
> written first so a side tree wins where both name a path, which is what having
> two trees means.
>
> **A pack's mods are identified for free.** A pack index names each file by URL
> and hash alone, with no project or version id — which would make a 150-mod
> pack list as 150 anonymous filenames and leave `content update` nothing to
> work with. But a platform's own CDN URL carries both ids
> (`cdn.modrinth.com/data/<project>/versions/<version>/…`), so `parse_file_url`
> recovers them at no cost, and one bulk `projects` call fills in every title
> and icon. Both are provider-trait methods, so CurseForge slots in behind the
> same seam. A file the source does not serve is recorded as `source: "file"` —
> it installs, it is simply not updatable, exactly like a local import.
>
> **What a pack cannot do:** it cannot be installed into an entry whose flavor
> or game version differs from what it pins (`ModpackEntryMismatch` names both
> sides — the entry's profile is resolved and neither can change in place), and
> a pack pinning a loader with no hestia flavor is refused by name rather than
> quietly installed as vanilla. The flavor check is the *registry*, not a match
> arm: a pack's loader name **is** hestia's flavor id, so adding a flavor needs
> no edit in the modpack flow.
>
> **A pack update carries the game version with it**, because that is what
> updating a pack means — a pack that bumps 1.21.1 → 1.21.4 is the common case,
> and refusing it would leave `modpack update` useful only for the rare
> same-version bump. So it runs the entry's existing version-update flow (a
> server's automatic pre-update backup included) behind the same explicit
> `allow_downgrade` gate. A loader change still refuses: the flavor is baked
> into the resolved profile.

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

> **Enable/disable, update-check, and pin extend the same model.** Beyond
> add/list/remove/update, an installed item carries an `enabled` flag
> (`content.json`, defaulting true so old records decode enabled) toggled by
> `server|instance.content.enable`. Disabling is enforced at the *single*
> point the flag needs: the launch-time mirror `sync` treats a disabled item
> like a profile non-member — kept out of `data/` — so a disabled mod is never
> loaded and a backup restore can't resurrect it; the toggle also applies the
> filesystem change immediately (the entry is stopped) so the state is visible
> before the next start. A datapack has no mirror, so it disables by the
> standard `.disabled` rename inside its world (Minecraft ignores the suffix),
> which the world backup carries. `content.check_updates` is a *separate*
> on-demand call (not baked into `list`): it resolves each platform item's
> newest compatible version upstream and reports which differ, keeping `list`
> fast and offline. `content.set_version` re-pins one item to a chosen
> published version — the update path with an explicit pin instead of "newest"
> — as a `ContentManager` job like `update`. All three refuse a running or busy
> entry, matching the existing content ops. The desktop reaches every content
> operation through the generic bridge; a local-file import uses
> `tauri-plugin-dialog`'s native picker to hand the daemon a real
> daemon-readable path (a webview `File` has none).

> **A local-file import is inspected, not trusted.** A picked path used to be
> copied blind under whatever kind the browse chip happened to show — a
> resourcepack staged under the "mods" chip landed in `mods/` and silently
> broke the game, and a `.mrpack` passed the filename check to install as a
> garbage mod. The daemon now reads the archive's central directory
> (`content/inspect.rs`, the `zip` crate already in-tree) and classifies it:
> `content.inspect(path)` returns the detected kind, validity, and a reason.
> Detection is **loader-agnostic** — a mod is any loader's manifest
> (`fabric.mod.json`, `quilt.mod.json`, `META-INF/mods.toml`,
> `neoforge.mods.toml`, …), so a new flavor is one more entry in the manifest
> table, not a code path; a datapack vs. resourcepack is disambiguated by the
> `data/` vs `assets/` tree under a shared `pack.mcmeta`. The desktop inspects
> each file on pick, defaults its kind to the detected one, and the **review
> step carries a per-file kind override** (constrained to the kinds the target
> accepts) plus the install destination — so the detected kind is a suggestion,
> not a verdict. On `content.add` the daemon hard-rejects only what genuinely
> cannot be single-file content (an unreadable archive or a modpack) and
> otherwise **honors the requested kind**, so a review override installs where
> asked; a requested/detected mismatch is logged, not blocked. An unrecognised
> but valid zip is installable once the user picks a kind. This is a
> desktop/daemon surface (the CLI passes an explicit `--kind` already); the
> `.mrpack` extension is dropped from the picker, since a modpack installs at
> instance-create, not here.

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
> With `saves/` linked (linked sync), an instance's datapack lives in the
> *shared* world every instance opens — the pack itself is visible
> everywhere, while its `content.json` provenance stays in the installing
> instance, so other instances list it as untracked world data. Known
> behavior, not a bug: the world carries its own datapacks, and exactly one
> instance manages each.

> **A content profile is a selection, not a copy.** An instance's profiles
> (`profiles.json` beside `content.json`; absent = no profiles) are named
> subsets of the managed pool, keyed by **filename** — the one index field
> always present and unique (`project_id` is empty for local imports). The
> managed dirs stay the single source of truth: activating a profile changes
> only what the launch-time reconcile mirrors into `data/` — members are
> mirrored, tracked non-members have their `data/` copy removed (the managed
> copy stays), and untracked files are never touched, consistent with the
> untracked-not-adopted rule. No profile active = mirror everything — exactly
> the pre-profile behavior, so existing instances need no migration. Selectable
> kinds are mods, resourcepacks, and shaders only: a datapack *is* world data,
> outside the pool. Worlds, `servers.dat`, and all other game data are shared
> across profiles *by construction* — every profile runs against the same
> single `data/` (per-profile game dirs and symlinked game dirs were rejected).
> The pool keeps profiles honest at its edges: removing content prunes the
> filename from every profile, and a content update remaps a member to the new
> version's filename. When sessions are already running, a launch skips the
> reconcile entirely (the mirror is in use; jars are locked on Windows) and a
> profile override that differs from the active one is refused. The `none`
> name is reserved: `launch { profile: "none" }` overrides an active profile
> with "no profile" for one launch. Servers have no profiles.

> **A global profile stores project references, never jars.** A data-home-level
> profile (`profiles/<name>.json`, a bare array of `{source, project_id, slug}`
> — the disk is the registry, the name is the slugged filename) is a reusable
> "starter pack" of content: jars are version- and loader-specific, so each
> `instance.profile.apply` resolves every reference against the *target*
> instance's game version and loader through the ordinary add-content path
> (`pick_version`, dependencies included). Applied content becomes an ordinary
> pool item with an `origin` tag (`profile:<name>`), so all downstream
> machinery — the mirror, backup heal, untracked detection, update — works on
> it unchanged (an update preserves the tag; a user re-install clears it, taking
> ownership). Apply is **one-shot and additive**: a reference already in the
> pool is skipped (the local copy wins), one with no compatible version is a
> per-item failure the batch continues past, and de-listed references are never
> removed (the launch-time reconcile stays a per-instance-profile concern —
> `content list` shows the origin instead). Removing a profile-tagged item
> locally is refused naming the profile — it would silently reappear at the
> next apply; the reference leaves the global profile instead. The apply runs
> as a `ContentManager` job under the instance's in-flight key, publishing the
> `content.*` topics.

> **Settings capture is opt-in per profile, and scopes only settings.** An
> uncaptured profile inherits the global `shared/` store; `capture` snapshots
> the settings-class sync targets into the profile's own store
> (`<instance>/profiles/<name>/`, whose existence *is* the captured flag —
> disk-is-the-registry, like `java` and `backups`) and from then on launches
> under that profile sync against it. Divergence after capture is by design;
> `release` deletes the dir and the profile inherits the global store again.
> Under linked sync the two target classes capture differently: the `config`
> **folder repoints the link** — `data/config` links into the profile store
> instead of the global one, so in-game settings changes write through to the
> captured store and never touch the global one — while `options.txt` keeps
> the per-scope **copy-reconcile** with the same merge rules. `saves` and
> `screenshots` always stay on the global store: capture forks *settings*,
> not game data (worlds stay shared across profiles by construction). The
> stale-link relink handles every scope switch, because a profile store path
> counts as a hestia store target (`…/profiles/<name>/<rel>`); capture and
> release require a stopped instance — a live session's `config` link writes
> through the store being replaced. A profile rename moves its captured dir;
> a profile removal deletes it.

> **Skins follow Modrinth's shape, minus its couplings — and skip the CLI.**
> Skin management (`skin.*`/`cape.*`) is a desktop-only surface: picking a skin
> is visual, so the CLI deliberately grows no command for it. The design mirrors
> Modrinth's launcher where its rules earn their keep: a local library preserves
> textures (before any change, the currently equipped skin is saved into the
> library if neither it nor a default already records it — switching away from
> an externally-set skin must never lose it), library rows are keyed by Mojang's
> texture hash (an upload response reports the minted key and the row follows
> it), and the vanilla defaults are listed by their public texture URLs rather
> than bundled PNGs (equipping one is a by-URL skin change). It deliberately
> drops two Modrinth couplings: the library is **global**, not per-account
> (a texture is not an entitlement; the equipped state is per-account already),
> and a cape is **not** bound to a skin — Mojang's own API models them as
> independent (`skins/active` vs `capes/active`), and binding them is what
> forces Modrinth's save-row reconciliation dance. Changes apply immediately
> (no debounce): the daemon is resident, so there is no app-close edge to flush.

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

> **Sync links folders and copies files — Pandora's split, adopted.** Sync
> was originally all-copy ("copied, not symlinked"): each instance kept its
> own physical copy of every target, reconciled newest-wins at launch. That
> call was revisited for one reason — worlds. Copying `saves/` across
> instances would duplicate gigabytes per instance and still leave each copy
> divergent; linking stores a world **once** and shares it instantly. So
> folder targets (`saves`, `config`, `screenshots`) are now **links** into
> the flat `shared/` store (a symlink on POSIX, a junction on Windows —
> junctions need no privileges), while file targets (`options.txt`
> key-merged, `servers.dat`) keep the copy-reconcile: file symlinks need
> elevation or developer mode on Windows, and merge semantics need a real
> copy anyway. The original decision's three objections each found a
> narrower home instead of blocking linking wholesale: concurrent live
> servers → servers are decoupled from sync entirely (a server's shareable
> state is its own `server.config.*` and `server.properties`, never a
> cross-entry store); content ownership → the managed content dirs are still
> rejected as targets (per-instance selection is impossible over a shared
> dir); backups archiving through links → instance backups no longer exist.
> The safety story was Pandora's **empty-or-linked guard**: a folder became
> a link only when missing, empty, or already linked into a hestia store —
> a non-empty real directory was never touched, only surfaced as
> `cannot_link` until an explicit `sync adopt` moved its entries into the
> store (all-or-nothing per target, refused on any name collision). That
> guard is now **narrowed to the collision it was really about** — see the
> warning-noise decision note below: a folder holding only the instance's
> own files is adopted automatically, since moving it can destroy nothing,
> and only a name the store already has stops it.
> Only links pointing into a hestia store (`…/shared/<target>`) are ever
> touched, so a user's own symlinks survive; a stale store link after a
> data-home move is relinked at the next launch. Pack selection
> (`options.txt`'s `resourcePacks`) stays entry-local — merged like
> Pandora's, but never pushed to the store. **Accepted risks, documented
> not guarded:** two instances (or sessions) opening one shared world are
> arbitrated only by Minecraft's own `session.lock`, and instances of
> different versions/loaders writing one world can corrupt it — plus, until
> import/export lands, instance data (the shared worlds store included) has
> no backup story at all. Any code that walks or deletes an instance's
> `data/` must treat a link as a boundary, never a directory to descend
> into — `remove_dir_all`'s link-preserving behavior is pinned by a test.

> **The id is an opaque uuid; the directory is the slug — decoupled.** Two
> facts about an entry pull in opposite directions: its *internal key* must
> never change (the supervisor's process key `server-<id>`, the port-claim and
> content in-flight key, and how the on-disk `processes/<id>/` records are keyed
> — a change orphans a running process and every record pointing at it), while
> its *on-disk directory* should read like the entry and track its name. Binding
> both to one `<slug>-<suffix>` token forced a choice; splitting them removes it.
> The `id` is now a bare UUIDv7 hex string minted once at create — opaque,
> stable, never a path component (`registry::allocate_id`). The directory is
> named `slugify(name)` (`registry::dir_name`), unique because `name_taken`
> forbids two entries slugging alike. So `rename` rewrites the `name` **and
> moves the directory** to the new slug, while the id — and everything keyed by
> it — stays put; it is guarded stopped-and-not-busy, since no live process may
> hold the folder mid-move. A front-end still targets an entry by its **name**,
> never the id: a reference resolves by exact id *or* any spelling that slugs to
> the display name (`My Server`, `my-server`, `MY  SERVER` all hit the one
> server named "My Server"). That rule — `proto::naming::reference_matches` —
> lives in `proto`, the same no-drift seam the wire payloads use, so the daemon
> (`get`) and every front-end resolve a reference identically; it is unambiguous
> only because `name_taken` keeps slugs unique. This is possible cheaply because
> the id was never *derived* from the directory name — `registry::scan`
> deserializes it from the record JSON — so the folder is just a container the
> resolvers (`server_dir`/`data_dir`) name from the record's current name.
> The rejected alternative was the old scheme — id *equals* the slug, so it
> could not move on rename; the directory then lied about the entry's name
> forever (`servers/smp-3f9a2c7d/` lingering after a rename to `cozy`).

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
> warnings) already marks it as risky. Backups are **server-only**: an
> instance is an interactive client session with no RCON channel to quiesce
> it and no analogue of a long-running server's unattended schedule, and
> archive/restore proved the wrong tool for it — instance **import/export is
> the intended replacement and is deferred**, so until it lands instance data
> has no backup story at all (the instance `update` downgrade warning states
> that nothing is backed up).

> **A world describes itself; a directory listing does not.** `instance.worlds`
> began as a `read_dir` of `data/saves/` returning folder names, which is all the
> datapack picker it was written for needed. But a folder name is not the world:
> the player names a world in-game, so `saves/New World (2)` may be "Hardcore
> Attempt 4", and only the save knows which version wrote it, how it plays, or
> when it was last opened. So a world is read from its own `level.dat` (gzipped
> NBT, via `fastnbt`) — `minecraft/world.rs` — and `WorldInfo` carries the
> display name, version, game mode, difficulty, hardcore and cheat flags, last
> played, footprint, and the world's own `icon.png`.
>
> Two rules keep it honest. **The folder stays the identity**: every operation
> still addresses a world by folder (a datapack installs into
> `data/saves/<folder>/datapacks/`), because that is what the game reads and what
> the content index keys on — the display name is presentation, and two worlds may
> share one. And **every field but the folder is best-effort**: saves span more
> than a decade of formats, an old one carries no `Version`, a corrupt or
> half-written one cannot be parsed, and a world a running game is flushing may be
> caught mid-write. None of that may hide a world from a listing, so a failure
> yields the folder alone with `read: false`, and a front-end says "could not be
> read" rather than rendering defaults as facts. The icon is **inlined as base64**
> rather than served as a path: the alternative is widening the webview's
> asset-protocol scope to the data home, which also holds `accounts.json`.

> **An unfinished record says *which* kind of unfinished, so recovery can act on
> it.** A server is registered before provisioning starts, because the record is
> what holds its port claim through a long download; `provision_server` removes
> it if the pipeline fails. But that cleanup only runs while the daemon that
> started the create is alive — kill it mid-create and the half-registered record
> survived every subsequent start, `start` correctly refusing it ("still
> provisioning") while nothing ever reconciled or removed it. A permanent
> un-startable orphan, holding a port.
>
> The fix is not "register later": the claim genuinely needs the record first.
> It is that `ready: bool` could not say what recovery needs to know. A record
> now carries a **`ServerPhase`** — `Provisioning`, `Ready`, `Updating` — and
> `Engine::recover()` reconciles at startup beside `ProcessSupervisor::recover()`
> and the temp-artifact reclaim: no job survives a restart, so a `Provisioning`
> record belongs to a create that will never finish and is **discarded**,
> reaching the same conclusion the live failure path does.
>
> The distinction earns the enum. `Updating` is *also* not-ready, but it belongs
> to a server that was ready before — its world is on disk, so discarding it
> would destroy real data. It is kept and logged, and updating again finishes it,
> which is what the update path's own gate already promised. A single boolean
> conflated "nothing here is yours yet" with "your world is here, mid-swap";
> recovery cannot be safe without telling them apart, and a test pins both
> outcomes.

> **A temp artifact is only valid while its job holds the claim — so a restart
> invalidates every one.** Write-through-a-temp is used all over: the backup
> archive's `.part`, the downloader's `.part`, the Java installer's `.staging`
> tree and its downloaded archive. The convention assumes the process that
> created one either finishes or cleans up — precisely what a crash breaks, and
> nothing reaped them: killing the daemon mid-archive correctly refused to
> promote the partial backup (the rename *is* the commit, so `backup list` stayed
> clean) but left `20260725-093946-manual.tar.gz.part` in `backups/` forever, and
> the same hole existed for `.staging`. Stating it as one invariant fixes the
> class rather than the file: an artifact is valid only while its job holds the
> matching in-flight claim (`InFlight`, `runtime/managers/job.rs`), and no claim
> survives a restart — so at startup every artifact still on disk is abandoned by
> definition. `engine/reclaim.rs` is that pass, composed by
> `Engine::reclaim_temp()` and called from the daemon's boot beside
> `ProcessSupervisor::recover()` (and again after a data-home change, since the
> new home's artifacts are no more claimed than the old one's). It is deliberately
> **not** recursive — each subsystem knows the one directory its artifacts land
> in, and walking a data home whose asset store is six figures of files is not
> something to pay at every start. A backup additionally reclaims before writing,
> so the store is tidy whether or not a restart intervened. The downloader's own
> `.part` files need no sweep: they sit at the destination and a retry truncates
> them, so they are self-healing rather than accumulating. Reclaimed bytes are
> logged, so the leak is visible rather than silent.

> **The properties schema is generated, not maintained — and it is not the
> file.** `config set` validates a `server.properties` key against a *schema*
> derived from the server binary itself, never against a curated key list. A
> hand-kept list is a per-version maintenance liability (keys appear,
> disappear, and differ across the versions Hestia launches; the list would
> silently rot). So the create job runs the freshly downloaded server once in a
> **throwaway directory** (`<entry>/.schema/`, discarded after): with no
> `eula.txt` there the gate makes it emit a complete `server.properties` (every
> key + default for exactly that version, mods included) and exit almost
> immediately, before binding ports or generating a world. That pristine file is
> stored beside the record as `schema.properties`, and its keys are what a
> `config set` is checked against; the value is written into the game's live
> `data/server.properties`, which is seeded with any schema key it lacks.
> Pre-1.7.10 servers have no EULA gate and would boot for real, so the run is
> killed after a 60 s timeout. A version update reruns it the same way and
> replaces the schema.
>
> The run is deliberately *outside* `data/`, because the two things it used to
> conflate are different: the schema is "the keys this version knows", the file
> is "the values this server holds". Running in `data/` meant the server
> round-tripped the existing file, so what came back was the current values, not
> a key set — and **vanilla preserves keys it does not recognise** (verified
> against 1.21.1: an unknown key seeded before the run survived it). A key
> retired by a version update therefore stayed in the file forever and, because
> the file was the validation source, stayed settable forever. Now it stays in
> the file (it is a value, and silently deleting lines the user or a mod may own
> is worse than the drift) but is no longer in the schema, so it reads and lists
> while refusing further writes. Deriving the schema separately also makes the
> no-schema fallback explicit rather than accidental: schema generation is
> best-effort — a failure is a *warning*, not a create failure — and a server
> with no `schema.properties` accepts any unmanaged key rather than rejecting
> every one. `Servers::has_schema` is that state, so a caller can report it
> rather than leaving the user to discover that this server validates nothing.

> **A degraded outcome rides on the result, never only in the log.** Several
> steps in this codebase are deliberately best-effort — the properties schema
> run, each `sync` target's reconcile — because failing the whole operation over
> them would be worse than proceeding. But "proceed" was implemented as a
> `tracing::warn!` and an unqualified success to the caller, so the user learned
> nothing: a 1.6.4 server was created with no validatable property schema and
> said only "created", and an instance whose `saves` could not be linked launched
> and played against the *wrong world* with the only trace in the daemon's log.
> The daemon's log is not where a user finds out what just happened to their
> data.
>
> So a degraded outcome is part of the operation's **result**:
> `proto::warning::WarningInfo` is a structured, exhaustive enum — the exact
> shape and discipline as `ErrorInfo`, no prose authored at a call site — carried
> on the job done events (`server.create.done`, `server.update.done`,
> `instance.launch.done`) and on the standing views that stay true afterwards
> (`ServerDetails`, so `server info` keeps saying it long after the create
> scrolled past). `Sync::apply` therefore *returns* its warnings instead of
> logging them, and the empty-or-linked guard reports which arm refused
> (`NotSharedReason`). Every variant carries a `hint()` beside its `Display`
> headline: a warning the user cannot act on is noise, so the remediation is part
> of the type rather than something each front-end invents. Front-ends get it for
> free and localize it generically — the CLI prints a `warning:` line plus the
> hint (`View::Warning`), the desktop renders `warning.kind.*` /
> `warning.hint.*` message keys as a toast on the operation and a standing
> `WarningNotice` on the entry.
>
> The rejected alternatives were raising the log level to WARN (same invisible
> place) and hard-failing the operation (refusing to launch over a leftover
> folder, or refusing to create a server whose schema run timed out — both break
> a recoverable situation). A front-end must not have to *ask* whether the thing
> it just did worked properly.

> **A warning the user did not cause is a bug in the launcher, not a notice.**
> The rule above earned its keep and then over-fired: the two warnings a normal
> user actually met were both about hestia's own limitations. Every NeoForge
> create said its property schema could not be derived (structural — see the
> NeoForge note), and an instance whose `data/config` had contents — which a
> modpack's `overrides/` puts there before the first launch ever runs — said
> `config` was not shared, pointing at an `adopt` chore hestia could perfectly
> well do itself. Neither followed from anything the user did, and the second's
> "remediation" was work the daemon was declining to do.
>
> So the fix in both cases was to **remove the degradation**, not to soften the
> text. The schema run stopped using an argument file it could not resolve; the
> folder guard was narrowed from "never touch a non-empty directory" to "never
> overwrite" — a folder holding only the instance's own files is adopted at the
> launch that would have warned, silently, because moving files into the store
> is exactly what making it a target asked for and nothing can be lost. What
> survives is a warning about a **name clash** (`NotSharedReason::Collides`),
> which the user must resolve because either copy could be the one they want,
> and a foreign link, which is theirs to repoint.
>
> Two rules keep the automatic pass honest. **A modpack owns its config tree**:
> a pack ships `config/` as part of what it is, so folding it into the store
> every other instance reads would push one pack's settings onto all of them —
> `Settings::Local` leaves those folders alone, with no warning, since it is a
> deliberate outcome rather than a degraded one. It is the automatic pass only:
> an `adopt` the user asks for still opts the folder in, and the link it leaves
> is reconciled from then on, so a pack *can* share if that is what the user
> wants. And **hestia never breaks a link it did not just make** — a folder
> already sharing keeps sharing, whatever else changes.
>
> Sharing is now switchable outright (`sync.enabled`, the config store like
> `announcements.enabled`): moving a user's files into a common store is a
> policy some people simply do not want, and the honest answer to that is a
> switch, not a warning they cannot turn off. Off, no pass runs — and existing
> links are left exactly where they are.

> **Everything serialized is camelCase, except the `config.*` key vocabulary
> and upstream DTOs.** The wire is camelCase (`proto` structs carry
> `rename_all = "camelCase"`; `tests/casing.rs` enforces it), and the
> persisted engine records follow suit so on-disk JSON is uniformly camelCase
> (`ServerRecord`, `InstanceRecord`, accounts, `content.json`, the settings —
> Rust field names stay `snake_case`, only the serialized form is renamed).
> Two deliberate exceptions, neither machine-enforced in `engine` because a
> blanket guard would fight them: (1) the **`config.*` keys are a stable
> kebab-case CLI vocabulary** (`memory`, `jvm-args`, `backup-interval`,
> `defaults.jvm-args`) — the per-entry `JavaSettings`/`BackupSettings` decouple
> them with explicit key constants, and the global `Settings` navigation
> translates each dotted segment through `naming::config_key_to_field` (and
> `settings_to_config_keys` for the `config list` view), so a user keeps typing
> kebab while `config.json` stores `jvmArgs`; (2) the **upstream DTOs**
> (Adoptium, Mojang, Fabric, Modrinth, Microsoft) keep whatever casing the
> remote API uses, since they deserialize *its* JSON, not ours. The proto
> guard covers the one contract that matters — the socket; engine record
> casing is a convention, not a lint, precisely because the DTOs next to those
> records legitimately aren't camelCase.

Errors are `thiserror` enums (e.g. `ConfigError`); the daemon maps them to
`ipc::errors` codes at the service boundary. `anyhow` is used where an operation
composes many fallible steps (accounts, minecraft, java, provisioning).

## `daemon` — hestiad

The resident core: it owns the IPC endpoint, routes requests to handlers, and
manages autostart. The only crate that links `engine` — including its process
supervisor, which the daemon drives (`recover()` at boot, `stop_all_and_wait()`
at shutdown) and reaches through `runtime.processes()`.

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
      `server.create` / `instance.launch` / `server.backup.create|restore`
      answer immediately while the blocking engine work runs off-thread,
      publishing progress/done/error events through the hub (the two backup
      job types share the `backup.progress|done|error` topics, disambiguated
      by job id).
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
    - **`event_hub.rs`** — `EventHub` fans daemon events out to subscribed
      connections, filtered by job id, and unsubscribes them on disconnect.
- **`services/`** — the single wire-in point, one registrar per domain
  (`lifecycle`, `config`, `cache`, `java`, `download`, `accounts`, `skins`, `process`,
  `server`, `instance`, `backup`, `content`), each registering its channels with
  one `on.handle::<C>(…)` apiece; `services/mod.rs`'s `make_router()` is the list
  of `register()` calls, and `services/guards.rs` holds the preconditions the
  registrars share (`find_server`, `is_running`, `ensure_stopped`,
  `ensure_no_backup|update|content`, `require_backup`). Today: `health.ping`, `app.info`,
  `daemon.status|stop` (stop takes `stop_processes`; without it supervised
  processes keep running — the *front-end* decides which, see the decision note
  below), `config.get|set|list` (the reserved `home`/`autostart` keys
  routed to the path pointer and login registration), `cache.info|list|clear`,
  `java.releases|list|install|uninstall`, `download.start`,
  `account.login.begin|login.complete`, `account.list|switch|remove` (`switch`
  picks the default account launches use; `list` reports it),
  `skin.list|add|update|equip|reset|remove` and `cape.equip|clear` (the desktop
  skin picker: one `skin.list` answers the merged library/defaults/external
  skins plus the owned capes; `update` edits a library entry's label/arm style,
  re-pushing an equipped skin's variant; changes relay to Mojang with the
  account's token),
  `process.start|stop|list|status|logs`, `events.subscribe`,
  `server.flavors|versions|resolve`,
  `server.create|update|rename|list|status|info|remove|start|stop|logs|command`
  (create
  requires the caller to assert EULA acceptance; update refuses a running or
  still-creating server and, without `allow_downgrade`, a downgrade — a
  front-end updates a running server by explicitly stopping and restarting it
  around the job, the CLI's confirmed stop-update-start; rename rewrites the name
  and moves the directory to the new slug (the id is untouched), refused while
  running or busy — see the decision note below;
  start/stop/status/logs are thin over
  the supervisor, merging the stored record with live process state; `info` is
  the static, informational view — descriptor, on-disk locations, and the disk
  footprint (a directory walk), deliberately kept off the live `status` call;
  command
  relays one console command over the running server's rcon channel),
  `server.config.get|set|list` (the reserved `memory`/`jvm-args`/
  `backup-interval`/`backup-retention` keys on the record plus any
  `server.properties` key, bar the hestia-managed ports/rcon ones),
  `server.backup.create|list|restore|remove` (create archives a running
  server live; restore refuses a running or busy server and verifies the
  backup exists before answering with the job id), and the `instance.*`
  counterparts:
  `flavors|versions|resolve|create|update|rename|list|info|remove|worlds`
  (`worlds` describes a client's save worlds from each one's own `level.dat` —
  see the decision note below; `info` is the
  static, informational view — descriptor, on-disk locations, and disk footprint
  — the instance twin of `server.info`), plus
  `instance.launch|stop|logs` (concurrent sessions are opt-in — `launch`
  refuses a running instance unless `new_session` is set, then each launch is
  a new session; `stop` fans out to every session or a named one; `logs`
  targets the newest running or a named session — all thin over the
  supervisor), and `instance.config.get|set|list` (`memory`/`jvm-args`
  only), and `instance.profile.list|create|remove|rename|use|edit|capture|release`
  — the per-instance content profiles (CRUD is metadata-safe while running and
  applies at the next launch; `create` guards the pool read behind the
  content in-flight key when seeding; `capture`/`release` move the profile's
  settings store and require a stopped instance; `launch` takes a per-launch
  `profile` override, refused on a running instance when it differs from the
  active one — a desktop/daemon surface with no CLI verbs, like skins).
  Plus `sync.get|set|status` and
  `instance.sync.adopt` — the instance-only shared-config target set (`set`
  validates each path: relative, no `..` escape, not a launcher-managed
  dir; `status` reports each instance's folder link states; `adopt` moves a
  stopped instance's existing folder contents into the shared store). Plus
  `content.sources|search|project|versions|modpack.resolve` — thin over the
  engine's content registry (an empty `source` selects the default; search,
  project, and versions are plain request/response, and `modpack.resolve`
  downloads the `.mrpack` index inline, so the client facade calls it with a
  longer timeout) — plus the per-entry install surface
  `server.content.add|list|remove|update` and its `instance.content.*`
  counterpart (add/update are jobs over a `ContentManager`, publishing the
  `content.*` topics; list/remove are plain request/response; all refuse a
  running or busy entry). Plus
  `server.modpack.install|update|status|remove` and its `instance.modpack.*`
  counterpart — install and update are jobs over a `ModpackManager` publishing
  the `modpack.*` topics, keyed by the entry's process id like every other
  per-entry job; an install that *creates* its entry has no key to conflict
  with and claims its own job id instead. Split per side rather than taking one
  target-tagged channel, so the router's account gate covers the instance half
  by prefix. Plus `profile.list|create|remove|edit` — the global
  content-profile reference lists (edit resolves adds through the content
  registry) — and `instance.profile.apply`, a `ContentManager` job installing
  a global profile's references into a stopped, non-busy instance.
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

> **Instances are gated on a signed-in account, in the router.** A user cannot
> use — let alone play — Minecraft they do not own, and a stored account already
> proves ownership (sign-in resolves the game profile). So the whole instance
> surface is refused until an account exists: `Router::route` checks a channel
> prefix (`instance.*`, plus the instance-only `sync.*`) and answers a new
> `unauthorized` code before dispatch when `accounts().has_account()` is false.
> The gate lives at the router rather than in each handler for two reasons: it is
> a whole-domain lockdown (a per-handler `require_account` across ~30 handlers is
> error-prone and easy to forget on the next channel), and prefixing covers
> `instance.content.*` and `instance.profile.*` in their own service modules
> without touching them. Catalogue reads shared with servers (`content.*`) and
> the global `profile.*` reference lists stay open; only `instance.profile.apply`
> (instance-prefixed) is gated. Every front-end inherits the gate for free — the
> CLI surfaces the `unauthorized` message, and the desktop pairs it with a
> route-guard redirect, a library sign-in overlay, and a first-run prompt.

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

> **A job is cancelled by asking, at safe checkpoints — never by disconnecting.**
> Ctrl-C killed the CLI while the daemon ran the job to completion: a JDK landing
> minutes after the user stopped waiting, with no way to abort a download, an
> assets materialize or a backup from any front-end. The tempting fix — have the
> daemon cancel a job when its requesting client disconnects — is exactly the
> coupling the supervisor design removed, and would kill a legitimate background
> install the moment a terminal closed. So cancellation is an **explicit act**,
> like stopping a workload: one `job.cancel { id }` channel, keyed by the job id
> the job's own events already carry, so a front-end cancels the run it started
> whatever kind it was. The CLI turns a terminal interrupt into that request
> (`commands::cancellable`), which is the only reason Ctrl-C now stops anything.
>
> Inside, cancellation is **cooperative and checkpointed**, never a kill:
> `engine::Cancel` is a flag, and `engine::Job` carries it alongside the progress
> reporter — the two travel to the same places because a step that reports
> progress is exactly a step that can be stopped between reports. The checkpoints
> are the boundaries the staging discipline already created: per chunk in a
> download, per file in a library/asset batch and in a backup archive, and
> between pipeline phases. Stopping at one leaves precisely what a network
> failure at the same point would have left, so the existing failure paths do the
> cleanup — a cancelled Java install stages and never renames, a cancelled create
> discards its record exactly as a failed one does, a cancelled backup leaves a
> `.part` that `Engine::recover()` reclaims. Nothing new had to learn how to tidy
> up.
>
> A cancelled job is **not** an error. It settles on its own `<family>.cancelled`
> topic (every family names its terminal topics alike, so the drivers derive it
> from the done topic), surfaces as `IpcError::Cancelled` / `JobCancelled` rather
> than a daemon error, and is logged at info. A front-end that rendered
> cancellation as a failure would be blaming the user for what they asked for.

> **Supervision is engine state, and one stop reaches the whole tree.** The
> supervisor lived in the daemon, which made two things impossible. Its
> directory is `<data_home>/processes/`, engine-owned like every other registry
> here, but `set_data_home()` could not repoint it — nine subsystems moved on a
> `config set home` and the supervisor kept writing to the old one. And every
> engine flow that shells out (NeoForge's processor chain, the
> `server.properties` schema run, a Spigot build) had to spawn a bare child,
> because it could not reach the supervisor at all — three ad-hoc spawns with no
> containment, no records and no adoption. Moving it into the engine settles
> both: the only thing it could not know is where its events go, and that is a
> one-method [`ProcessEvents`] sink the daemon supplies at boot, so the engine
> still does not know a socket exists.
>
> That merge exposed the bug the split had hidden. The supervisor started every
> process as its own group leader and then signalled the *pid*, so stopping a
> workload orphaned whatever it had spawned. Termination is now the tree —
> the negated pid on POSIX, a kill-on-close job object on Windows, since it has
> no group that cascades — for servers and game sessions as much as for builds.
> The regression test is `crates/engine/tests/process.rs`: it fails against the
> old single-pid kill.

> **Workloads outlive the daemon by design.** The supervisor originally spawned
> children with `kill_on_drop` and piped output, which killed every server and
> game session on a graceful daemon stop — and leaked them untracked on a crash.
> Now the daemon is restartable/upgradable under live workloads (the same reason
> Docker grew `live-restore`): stopping a workload is always an explicit act
> (`server stop`, `process.stop`, `hestia daemon stop --all`), never a side
> effect of daemon lifetime. The cost is honest bookkeeping — on-disk records,
> start-time identity checks, file-based logs — and one observable gap: an
> adopted process's exit code is unknowable.

> **A finished process is labelled, not merely unrecorded.** Two promises here
> could not both hold: "a terminal state keeps its logs for post-mortem" and "a
> startup sweep deletes recordless dirs". A finished process leaves *exactly* a
> recordless dir — `records::remove` drops `record.json` and leaves the logs — so
> the sweep destroyed the post-mortem at the next restart, and the guarantee
> survived only until then. The observable symptom was the odd one: a killed
> process lingered in `process.list` (in memory only) until an unrelated daemon
> restart, and that same restart deleted its logs. There was no state in which
> the list was clean *and* the logs existed.
>
> The bug was the sweep's criterion. "Has no `record.json`" was standing in for
> "is a stray", but it is also the normal resting state of a finished process. So
> the end is now **explicit on disk**, the disk-is-the-registry discipline used
> for java runtimes, backups and server records: on exit the record is replaced
> by a tombstone (`exit.json`: state, exit code, when it ended, and where its
> logs are, since that is not derivable once the spec is gone). The sweep then
> deletes only directories with **neither** marker — a true stray, hand-made or
> half-written — and `TOMBSTONE_KEEP` prunes the oldest finished ones, because
> "keep the logs" without stated retention means "grow forever" (count-based, as
> `backup prune` is).
>
> `process.list`/`status`/`logs` read terminal entries from the tombstones rather
> than from memory, so what the daemon reports about a finished process no longer
> depends on whether it has restarted since — including its logs, which is the
> whole reason for keeping the directory. A process that died while unsupervised
> is entombed by `recover()` at the moment it is noticed. The rejected
> alternatives were exempting process dirs from the sweep (they then accumulate
> forever, which is what the sweep exists to prevent) and adding a `process.clean`
> verb (asking the user to resolve a contradiction the daemon should not have).

> **Stopping the daemon has three meanings; the front-end picks one, the wire
> carries two.** `daemon.stop` takes a boolean `stop_processes`, but a user
> typing `hestia daemon stop` with a server running has not expressed either
> value — "stop the launcher" is genuinely ambiguous about the server, and both
> guesses are bad (killing it loses the world's unsaved state; keeping it
> silently leaves a process the user thinks they stopped). So the *third*
> meaning — **ask** — lives in the front-end, not the contract: the CLI prompts
> on a terminal, and when piped refuses and names both flags, so a script must
> say which it meant. With no workloads running there is nothing to decide and
> the stop is immediate.
>
> Each front-end therefore declares its meaning rather than inheriting a default:
> the CLI asks, the **tray's Quit** stops the daemon and leaves workloads running
> (a menu item cannot ask, and quitting a tray icon must not kill someone's
> server), and the **desktop's stop button** does the same for the same reason.
> The one thing none of them does is decide silently while pretending the wire
> default did it. This was drift, not design, until now: the CLI's help still
> claimed workloads "keep running unless `--all`", which described neither the
> prompt nor the refusal. The input-side twin of the
> decision above: a stdin pipe exists only between a parent and the child it
> spawned, so it cannot be re-established for an adopted process (and dies
> with every daemon restart). RCON is re-establishable TCP state — any daemon
> can connect to any running server it knows the port and password for, which
> the server's record persists. Log streaming needed nothing new for the same
> reason: output already lives in files, tailed into `process.output` events.
> One caveat is inherited from vanilla: rcon has no bind-address setting, so
> the listener is network-reachable and the per-server random password is the
> only barrier (it never appears in logs).

> **Following logs is scoped to the entry, not to one run of it.** `logs -f`
> used to resolve a *live process* first (erroring "not running" when there was
> none) and key the whole stream to it, so a stop ended the session and a
> restart needed a fresh invocation — the opposite of what file-backed output
> buys us. The subject is now the server or instance itself: the supervisor's
> keys are deterministic (`server-<id>`, `instance-<id>_<seq>`), so a front-end
> names an entry's process family from the entry id alone, running or not. Three
> pieces make that expressible. (1) The key vocabulary moved to
> `proto::naming` beside `reference_matches` — a front-end derives the same keys
> the daemon does, through the one no-drift seam. (2) An event subscription
> filter now covers the *session keys beneath* an entry key
> (`naming::process_in_scope`), which is what lets one subscription follow an
> instance across launches; job ids carry no `_`, so a job filter still matches
> exactly one job. (3) The client stream carries `process.started` as well as
> output and exit, so a follower can tell a restart from silence. A follow
> therefore starts against a stopped entry (backfill from the file, then wait),
> renders a state line where a run ends or begins, and keeps the same stream —
> in the CLI's fullscreen session, its piped `tail -f` form, and the desktop's
> log panels alike. Two deliberate exceptions stay process-scoped, because
> there the process *is* the subject: the attach that follows `play`/`launch`,
> and the rcon console (which can only drive a live server anyway). The rejected
> alternative was a reconnect loop in each front-end — it re-derives lifecycle
> logic per client and still drops the lines either side of the gap.

> **An instance runs many sessions; a server runs one.** A client can be
> launched more than once at a time — **opt-in**: `launch` still refuses a
> running instance by default, and the `new_session` param (`--new-session`)
> unlocks a concurrent launch, so the common case stays a single session and
> the safety rail is the default. Under the hood `instance-<id>` is no longer a
> single supervisor key — it splits into an *entry key* (`instance-<id>`, still
> the unit for the backup/update/content/rename guards and their in-flight sets) and
> a per-launch *session key* (`instance-<id>_<seq>`). Ids are `[0-9a-f]` (a
> uuid hex string), never `_`, so a session prefix `instance-<id>_` can't collide across
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
`sync`, `process`, `daemon` — each a module under `commands/` exposing a `Subcommand` enum and a
`run()`. A domain with many verbs is a directory whose `mod.rs` holds only that
grammar and dispatch, with one file per verb group: `server/` and `instance/`
split into their verb groups (`create`, `update`, `config`, `lifecycle`,
plus the server's `backup` and `console`) over a shared `entry` module, and
`content/` splits along
its own seam — `browse` (search a source) versus `manage` (install into an
entry). Global flags (`--verbose`/`--quiet`/`--home`) sit on the root; `--home`
is exported as `$HESTIA_HOME` and only takes effect when `hestia daemon start`
spawns the daemon (a running daemon keeps its own directory). No command
auto-spawns: `commands/connect()` and `connect_running()` require a running
daemon, and `commands/start()` (behind `daemon start`) is the one that spawns it.

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

> **Every daemon capability gets a scriptable verb, or a written reason it has
> none.** Diffing the channels registered in `daemon/src/services/` against the
> CLI grammar found four families with no verb. Three are deliberate and
> documented — `skin.*`/`cape.*` (picking a skin is visual), `profile.*` and
> `instance.profile.*` (desktop surfaces by design). Two were drift:
> `instance.worlds`, reachable only as a side effect of the datapack picker, and
> the whole `process.*` surface, which nothing but `daemon stop`'s internal
> workload check ever read. Both now have verbs. The distinction that matters is
> *stated intent*: a channel with no CLI verb is fine when the architecture says
> why, and a bug when it does not — so the audit is repeatable rather than a
> matter of taste. `hestia process` is deliberately the supervisor's own view,
> keyed by supervisor id, not a second way to drive an entry: it answers the
> questions the entry-scoped verbs structurally cannot (every workload at once,
> and a process whose entry was removed under it).

> **`-vv` buys wire visibility, not more volume.** The CLI advertised three
> verbosity levels but `cli` and `client` contained zero `trace!` statements, so
> `-v` and `-vv` emitted byte-identical output — a flag the binary could not
> honour. The fix was not to sprinkle `trace!` until the line count differs:
> that satisfies a test while leaving the level meaning "more, somehow". A
> verbosity level should buy a *capability*, and there is exactly one thing a
> client can show that the daemon's own logs cannot — **the wire**. So `-vv` is
> frames: each request and reply with its channel, correlation id, byte size and
> round-trip time, plus session open/close and the count of waiters a close woke.
> That is precisely what someone debugging a CLI-versus-daemon disagreement
> needs, it lives in one place (`client/src/session.rs`), and it is the same
> stream for every front-end that links `client`. Payloads are deliberately **not**
> logged — they carry access tokens and rcon passwords — so a frame reports its
> size, never its contents.

> **A state query answers through its exit code, not only its stdout.**
> `hestia daemon status` printed `stopped` and exited 0, so
> `if hestia daemon status; then …` was true whether or not the daemon was
> running — the exit code conflated *answering* with *affirming*. Flipping that
> one command to exit 1 would have been worse: it collapses "not running" into
> "the query failed", which is the distinction a script actually needs, and it
> leaves the next state verb free to invent its own convention. So the contract
> is stated once (`cli/src/exit.rs`, documented in docs/cli.md) in systemd's
> vocabulary: **0** did-what-was-asked / running, **3** answered and *not*
> running, **1** the command failed, **2** usage (clap's own). `dispatch`
> therefore returns an `ExitStatus` rather than `()`, and the two verbs that
> assert one subject's running-ness — `daemon status`, `server <name> status` —
> produce it; everything else maps to `Active`. Verbs that *describe* rather
> than assert (`info`, `sync status`, the lists) stay 0 deliberately: "inactive"
> is not a claim they make, and overloading them would make 3 meaningless.

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

A Tauri v2 shell hosting the React frontend in the root `frontend/`, wired to
the daemon through the same one-way boundary as the CLI: the shell reaches
launcher logic only through `client` (never by linking `engine`). The wiring
is one seam, `src/bridge.rs`: a shared `Client` held as Tauri managed state
(started at shell launch — `bridge::start` runs the `Client::start()` spawn
path once, so opening the app brings its daemon up; the sidecar sits beside the
exe where `client::spawn` looks, and the `start_daemon` command is the same
path behind the UI's start button), a single generic
`ipc_call(channel, payload, timeout_ms)` command forwarding through the
session's public `call_raw`, and event forwarding: on connect the bridge
claims the session's one event-callback slot, subscribes to *every* daemon
event (`events.subscribe` with an empty id), and re-emits each as a
`hestia:event` webview event. A watcher task notices a lost daemon between
calls, emits `hestia:connection` transitions, and passively reconnects to a
daemon that comes back — but *reconnection* never spawns, so a daemon stopped
during the session stays stopped until the user starts it. `ipc_call` itself
never spawns either, and while the connection is down it answers
`connection_lost` from the held state rather than attempting the socket: one
connect attempt per watch interval, whatever the webview asks for.

The typed surface lives in the frontend, `frontend/src/api/`: `core/` (the
`ipc_call` wrapper with the SDK's timeout defaults, the event bus, and a
`runJob` driver mirroring `Session::run_job` — client-generated job id,
subscribe-before-start), `types/` (`proto` wire types **generated** by ts-rs —
a flat `generated/` dir plus one per-module barrel, regenerated with
`scripts/gen-types.sh`; the feature-gated `#[ts]` derives never enter a
production build), and one module per domain mirroring
the client facades. Over it sits `frontend/src/queries/` — the TanStack Query
layer, one module per domain mirroring the API namespaces **1:1** so the UI
only renders. Each domain exports its **factories** —
`queryOptions`/mutation-options makers (`serverQueries.detail(id)`,
`serverMutations.start(id)`), the single source of truth a router loader can
preload through — plus a **thin hook per API function** over them
(`useServer(id)`, `useStartServer(id)`); keys come from one hierarchical
factory keyed by stable entry ids (never the renameable display name), with
an entry's sub-resources nested under its `detail(id)` prefix so one sweep
refreshes the whole entry, and detail queries seed from the list cache.
Long-running operations are **job mutations**: every one routes through a
global job store (`useJobs`/`useEntryJobs` — an activity surface sees every
in-flight job with live progress, surviving unmount and navigation), and
`useJobMutation` adds the local view (`progress`/`job` on the mutation
result). Freshness is belt-and-suspenders: mutations invalidate their own
key prefixes on settle (declared as data in each factory), and
`queries/invalidation.ts` maps terminal daemon topics to key prefixes so
changes made by the CLI, the tray, or a schedule land without polling — a
reconnect invalidates everything. Streaming is hooks too: `useConnection()`
(daemon connection state), `useDaemonEvent(topic, handler)`, and log hooks
that accumulate `process.output` events onto the fetched tail
(`useServerLogs(id, { follow: true })`), so components never touch the event
bus. [hooks.md](hooks.md) is the layer's usage guide — patterns, the job
store, and the full hook inventory. The desktop signs in over the **sisu**
flow: `account.login.begin` returns the Microsoft URL for the shell to open,
`account.login.complete` redeems the redirect's OAuth code.

> **The desktop bridge is one generic command, not a facade mirror.** The
> intended recipe used to be one `#[tauri::command]` per feature calling a
> client facade — a placeholder written before the shell was wired. Mirroring
> ~80 channels as Tauri commands would add a third naming seam (proto channel
> → Rust command → TS wrapper) that can drift from both sides while adding no
> safety: `invoke()` results are untyped JSON regardless, and the daemon
> already validates every payload through the wire contract (`bad_request`,
> `unknown_channel`). So the Rust shell is a thin pipe and the typed layer
> lives once, in TS, where the frontend consumes it — adding a channel to the
> desktop is a TS one-liner, no recompile. Forwarding *all* events over one
> subscription likewise sidesteps the SDK's one-callback-slot constraint: the
> desktop needs many concurrent listeners (several jobs, live logs, list
> invalidation), so multiplexing by topic and job id moves into the frontend's
> event bus, where many subscribers are natural.

> **Sign-in is the one bespoke shell command — it must be.** Microsoft sign-in
> over the sisu flow (`bridge.rs`'s sibling `commands/auth.rs`, one
> `account_login_sisu` command) is the deliberate exception to the generic-pipe
> rule above, for the same reason Modrinth's launcher makes it one: the flow
> opens Microsoft's sign-in page in a **native webview window** and completes
> only by reading that window's URL when it redirects to
> `login.live.com/oauth20_desktop.srf?code=…` — and a cross-origin webview's
> location is readable only from the Rust side, never from the frontend JS that
> spawned it. So the command drives the two ordinary daemon calls
> (`account.login.begin` with the sisu method → `account.login.complete` with
> the captured code) around a `WebviewWindowBuilder` window it polls, closing
> it on success and returning the stored account (or `null` if the window is
> dismissed before completing — a cancel, not an error). The window is
> frameless (`decorations(false)`, matching the app shell's chrome-less
> windows) and carries no capability entry (the default capability is scoped to
> `main`), because the external sign-in page needs no Tauri IPC; only Rust
> touches it. The device
> code flow keeps its plain `account.login.*` path over the generic bridge for
> the CLI — the webview dance is a desktop-only affordance layered over the
> same contracts, adding no wire surface. The player-head avatar the shell
> shows for each account is rendered from the public `mc-heads.net/avatar/<uuid>`
> service (helm overlay included, initials fallback), the same source Modrinth
> uses — the account list carries only `{uuid, name}`, so the head is derived
> from the uuid rather than round-tripped.

> **Front-end preferences are desktop-local, in the data home — not the
> daemon.** UI state (a dismissed first-run overlay, remembered view) is the
> front-end's concern, not the launcher's, so it never crosses the socket: the
> `prefs_list|set|remove` commands (`commands/prefs.rs`) read and write
> `<data_home>/prefs.json` directly, resolving the same data home the engine
> uses (`common::paths`, so `--home`/`$HESTIA_HOME`/the persisted pointer are
> honoured). This keeps UI state out of the engine's typed `config` store (which
> the CLI and every front-end would then see) and out of the webview's
> `localStorage` (wiped with the webview cache, and not a real file). A Tauri
> store plugin was rejected for keeping its own file in the app dir — an extra
> indirection when a direct write to the data home is simpler. The store is
> schema-less: the front-end owns its own keys, consumed through the frontend's
> `usePrefs` hook.

> **Offline is one state, not a failure per read — and the shell brings its own
> daemon up.** With no daemon running, every query failed, and three defaults
> turned that into a hot loop: `retry: false` leaves a query in `error`,
> TanStack's `retryOnMount` refetches it whenever a new observer mounts, and a
> refetch with no data reads as `pending` — which flips a page's `loading`,
> swaps its body for the skeleton, unmounts those observers, and remounts them
> to start again. Measured at ~150 IPC calls a second and ~600 renders a second
> on the library page, which is what the "flickering" was. Three changes settle
> it, one per layer: `retryOnMount: false` (recovery is the reconnect sweep in
> `queries/invalidation.ts`, not a mount); the bridge answers `connection_lost`
> from its held state while the daemon is down, so a burst of reads costs one
> socket attempt per watch interval instead of one each, and emits
> `hestia:connection` only on transitions; and the failures log at `debug`
> rather than `warn` on both sides, since an offline daemon is a state the
> status bar reports, not an error per call. The UI then says it once: an
> `OfflineOverlay` in the app shell carrying the daemon's own start action —
> the whole app is backed by the daemon, so there is nothing useful to do
> behind it. The shell also **starts the daemon at launch**
> (`bridge::start`) — opening the desktop is as deliberate a launch of Hestia
> as `hestia daemon start`, so the overlay is the exception, not the greeting.
> Reconnection still never spawns: a daemon stopped *during* a session was
> stopped on purpose.

### Tray (`tray`)

A resident system-tray helper, built on Tauri's own tray crates
([tray-icon](https://github.com/tauri-apps/tray-icon) + a
[tao](https://github.com/tauri-apps/tao) event loop; gtk/StatusNotifier on
Linux, native on Windows) and wearing the desktop app's icon (embedded from
`crates/desktop/icons/` at build time, so both front-ends share one face). The
menu is an **Open Hestia** action, a status header (version + running/stopped),
a start/restart action, a start-at-login toggle bound to the reserved
`autostart` config key, and a quit that stops the daemon too (supervised
workloads keep running, as with any daemon stop). A worker thread polls the
daemon every two seconds over the client SDK and reports state changes to the
event loop; menu actions travel the other way over an mpsc channel, so the UI
thread never blocks on the socket. Left-click launches the desktop shell (the
same as the **Open Hestia** item) — the `hestia-desktop` binary beside the
tray, spawned detached; a second launch re-focuses the running window rather
than opening another (see the single-instance note below).

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

> **The tray and desktop must not share a GApplication id.** On Linux both
> front-ends go through `tao`, which creates a `gtk::Application` with
> `ApplicationFlags::empty()` and registers it — so GApplication acquires the
> D-Bus name equal to the app id and enforces single-instance by name
> ownership. When the tray reused `common::app::ID` (the desktop shell's Tauri
> `identifier`), whichever process started second registered as a *remote*
> instance and never showed — the tray blocked the desktop and vice versa. The
> tray now registers under its own `common::app::TRAY_ID`
> (`…hestia.tray`), decoupling the two. Single-instance *within* each
> front-end is enforced deliberately, not accidentally: the tray by its
> per-endpoint runtime lock (above), and the desktop by
> `tauri-plugin-single-instance` — a second `hestia-desktop` (e.g. the tray's
> left-click) hands its args to the running instance and exits, and the plugin
> callback shows/unminimises/focuses the existing `main` window. So only one
> tray and one desktop ever run, and re-launching surfaces what is already open.

## What's built vs. pending

**Built end-to-end:** the workspace and its enforced dependency graph; logging,
identity, path resolution; the wire protocol and typed client SDK; the config
store; the download cache; Java runtime management (install/list/uninstall via
Adoptium); Microsoft account sign-in (device-code and sisu) with token rotation;
skin and cape management for signed-in accounts (a preserved local skin
library, the vanilla defaults, upload/equip/reset and cape selection over the
Mojang profile API — daemon and desktop layers only, no CLI);
the process supervisor; the Minecraft provider layer (flavors, versions,
and profile resolution — vanilla, fabric and neoforge on both sides, paper,
folia, spigot and bukkit for servers only, the last two compiled locally with
SpigotMC's BuildTools); server
management (create = fully provisioned: profile + java + jar + EULA, each
server on its own claimed port; start/stop/status/logs over the supervisor;
a console over rcon — one-shot `command`, followed logs, interactive
`attach`); instance management (create a
record, launch materialises client/libraries/assets and spawns the game as the
signed-in account, and can run several concurrent sessions each with its own
Log4j2-routed log); per-instance content profiles (named selections over the
installed pool, enforced by the launch-time mirror reconcile — daemon and
desktop layers only, no CLI); shared settings/configs across instances (instance-only
`sync`: `options.txt`/`servers.dat` copied and merged, `saves`/`config`/
`screenshots` linked into the `shared/` store — with `sync status` link
states and per-instance `sync adopt` migration); in-place version
updates for both (downgrades gated
behind an explicit confirmation, the existing data backed up automatically
first); backups for both (on-demand archive/restore with live progress — a
running server is archived under the RCON save-off dance — plus per-server
scheduled backups with retention pruning); the content provider layer
(Modrinth search/project/versions/modpack resolution) with per-entry
install/management — mods or plugins on a server depending on its flavor, plus
datapacks; mods/resourcepacks/shaders/
datapacks on instances, from a platform project, a source page URL, or a local
file, with required dependencies resolved and a `data/` mirror that heals across
backups (datapacks install into their world, which the world backup already
covers); **modpacks** installed into a new or existing server or instance from a
project, a URL, or a local `.mrpack` — the pack's own loader and version
building the entry, its mods joining the pool as ordinary origin-tagged content,
and its `overrides/` written into the game directory under a hash record that
keeps a user's edits through an update; the kind-first browse and management CLI
(`hestia mod search`, `hestia modpack install`,
`instance <name> mod add|list|remove|update`, `hestia search`); the CLI
front-end over all of it; **announcements** (a signed news/notice feed
published from `news/` in the repo, filtered per build by platform, channel
and version range — `hestia news` plus a desktop page, banner and
once-per-id dialog, switchable off with `announcements.enabled`); self-update
on both front-ends; and the system tray (spawned by every serving
daemon, quick actions for open/start/restart/autostart/quit, left-click
launches the desktop shell).

**Pending:** natives-classifier extraction for pre-1.19 clients (the resolver
skips legacy `natives-<os>` classifier libraries, so old versions launch
without their LWJGL natives) and the legacy (virtual) asset layout; and the
desktop UI over the wired shell (the daemon bridge and typed API/hooks layer
are in place; pages are not).

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

> **News and notices are one mechanism with a severity dial, not two systems.**
> The prior art splits them: MultiMC ships a `notifications.json` (targeted by
> platform, release channel and version range, with critical/warning/information
> levels) *and* Prism adds an RSS news bar, each with its own fetch, parse and
> render path. They were added years apart, and the duplication is the cost.
> Hestia carries one `announce` domain: an `Announcement` with a `severity` and
> a targeting set, where an untargeted `info` entry *is* news and a
> version-ranged `critical` one *is* a notice. Adding a severity is an enum
> variant; adding a targeting dimension is a field plus a line in `applies`.
>
> **Targeting stays off the wire.** The engine has already applied it, so
> `proto::announce::Announcement` is deliberately narrower than the feed's own
> entry — the same rule that put `accepts` on `ServerInfo` rather than letting
> each front-end keep a flavor table. A front-end renders what it is given.
>
> **Severity picks the surface, one each**, because intrusiveness should track
> urgency (Carbon's notification pattern, and NN/g's heuristic) and because two
> surfaces for one announcement means dismissing it twice: `critical` opens a
> dialog once per id, `warning` leaves a standing strip until dismissed, `info`
> gets the news page and an unread badge. This mirrors what this codebase
> already does for daemon warnings — a toast on the operation, a standing
> `WarningNotice` on the entry — so the banner is a placement of an existing
> component, not a new pattern. Dismissal is daemon-side (`announce/seen.json`),
> so "once" survives a restart and the desktop and CLI share one read state; the
> alternative, desktop `prefs.json`, would have left the CLI re-nagging about
> what the UI had already shown.
>
> **The feed is signed with its own key.** Announcements are display-only text,
> which is why no other launcher signs them — but hestia renders remote markdown
> with links in the same app that ships an updater, so a hostile endpoint could
> phish ("critical: get the hotfix at …"). It reuses the minisign verification
> the updater already had (lifted into `engine/signature.rs`), against
> `ANNOUNCE_PUBKEY` rather than `UPDATE_PUBKEY`: the announce workflow runs on a
> push to the default branch while installers are signed only from a release
> tag, so one key would put the installer-signing secret within reach of
> anything that can land a commit. A compromised announcement key can say
> things; it cannot ship code. **An empty key set fails closed** — a build with
> no compiled-in key shows no announcements rather than trusting what it was
> handed — and the cached document is re-verified on load, so it is trusted
> because it verifies *now*, the same rule the download cache applies when it
> re-hashes a blob on the way out.
>
> **Publishing is a commit.** `news/*.md` compiles (`scripts/announce.py`) into
> one document that CI signs and uploads to a standing `announcements` release
> tag — a dedicated tag, because `releases/latest/` would tie news to the
> release cadence and 404 on any release that omitted the asset. Validation is
> strict and loud: a reused id would silently hide a *new* announcement from
> everyone who dismissed the old one, which is the failure direction that
> matters, so the compiler refuses duplicates. A malformed version bound
> likewise reaches nobody rather than everybody.
>
> Two accepted limits, documented rather than guarded: the signature covers the
> feed text, not the images it references by URL (an image can change or 404
> after signing — worst case a wrong picture, never code execution), and
> `HESTIA_ANNOUNCE_ENDPOINT` waives the signature so a **debug** build can
> render a hand-written feed. That waiver exists only under
> `cfg(debug_assertions)` (a release build has no path to it), only for an
> explicitly overridden endpoint, logs at WARN, and an unchecked feed is never
> cached — so it cannot outlive the process that read it.
>
> The poll is the daemon's one **unprompted** outbound request (the update check
> is on demand), which is a real behaviour change for a resident process — so it
> is switchable, `announcements.enabled`, and `AnnounceListResult` carries
> `enabled` because an empty list means something different when the feed is off
> than when nothing is published, and a front-end cannot tell those apart from
> the list alone.

## Tests

- `crates/proto/tests/` — `wire` and `golden`: the envelope and contract encodings
  are pinned so a wire change is caught.
- `crates/engine/tests/` — `store` (config/cache/java/servers/instances
  persistence) and `auth_oracle` (the account sign-in state machine); launch-plan
  assembly (classpath, placeholder substitution, the per-session log-config
  injection) is unit-tested in `minecraft/launch.rs`, the Log4Shell-safe
  session config in `minecraft/log4j.rs`, the config reconciliation, folder
  linking/adopt, and `options.txt` merge in `sync/`, the Modrinth mapping and `.mrpack`/URL
  parsing in `content/modrinth.rs`, and content version-pick / reference-matching
  in `content/install.rs`. The per-flavor accepted-kind composition is pinned in
  `minecraft/provider.rs` and the JVM-args precedence in
  `engine/flows/server.rs`; PaperMC build parsing is unit-tested in
  `minecraft/meta/paper.rs`, the SpigotMC version index in
  `minecraft/meta/spigot.rs`, and BuildTools' output naming and narration
  filter in `minecraft/spigot/buildtools.rs`.
- `crates/daemon/tests/e2e.rs` — a client-to-daemon round trip over a real
  socket; the session-key prefix invariant is unit-tested in `runtime/mod.rs`.

Run the fast core with `cargo build -p cli -p daemon`, then
`cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.

## Recording a decision

When a non-trivial architectural choice is made, capture *what* changed and *why*
here, next to the structure it explains, so this file stays the single source of
truth rather than letting the reasoning drift into commit messages or chat logs.
