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
| Tray      | `tray`           | `tray`    | native per-platform   | placeholder                |

The daemon (`hestiad`) is the resident core. The CLI is the first-class,
fully-wired front-end; the desktop and tray are scaffolds (see
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
engine  → config·cache·download·java·accounts·minecraft            → proto, common          (daemon-only)
cli     → bin hestia          (clap + ratatui presentation)        → client, common, proto
daemon  → bin hestiad         (router, services, supervisor)       → engine, proto, ipc, common, client
desktop → bin hestia-desktop  (Tauri v2 shell)                     → (tauri)                 (+ frontend/)
tray    → bin tray            (placeholder)
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
  (built with [Bun](https://bun.sh/)) — desktop.

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
the profiles, `ProvisionProgress`) the `server` and `instance` domains share.
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
- **facades** (`facades.rs`) — one struct per domain, reached through a `Client`
  accessor (`client.java().install(21, …)`), mirroring the engine's domain modules
  on the other side of the socket. Facade methods are one-liners over `Session`:
  `App`, `Daemon`, `Config`, `Cache`, `Java`, `Accounts`, `Process`, `Server`,
  `Instance`.
- **spawn** (`spawn.rs`) — locates and launches the `hestiad` binary, then retries
  the connection until it is listening.

## `engine` — the launcher engine

Daemon-internal domain logic. **`Engine`** (`engine.rs`) is the aggregate root:
the daemon constructs exactly one and threads it through every request handler. It
resolves the data directory once and owns each subsystem as a member behind a
getter. Adding a domain is a module, a member, and a getter here — the single
growth point, with no change to the daemon's serve loop. `set_data_home()`
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
- **`servers`** / **`instances`** (`Servers`, `Instances`) — the persistent
  stores, one directory per entry beside a JSON record (`servers/<id>/server.json`
  holding the resolved profile snapshot; the disk is the registry, as with
  `java`). Each record also carries a `JavaSettings` (`minecraft/launch.rs`):
  the per-entry `memory` (one value driving both `-Xms`/`-Xmx`) and extra
  `jvm-args`, injected into the launch plan at each start/launch; the
  `config_get/set/list` methods validate and persist them (servers also pass
  property keys through to `server.properties` — a set must name a key the
  server's own generated file carries, so a typo cannot silently drift the
  file; the hestia-managed ports/rcon keys are rejected — see the decision
  note below). An entry directory holds the record beside `data/`, the game's
  own working directory; the root is reserved for the managed content
  directories (`mods/`, `plugins/` for servers / `resourcepacks/` for
  instances, `configs/`, `backups/`), each created on demand — see the
  decision note below:

  ```
  servers/<id>/               instances/<id>/
  ├── server.json             ├── instance.json
  ├── mods/ plugins/          ├── mods/ resourcepacks/
  │   configs/ backups/       │   configs/ backups/
  └── data/                   └── data/
      jar, libraries/,            saves, options, logs —
      eula.txt,                   the game dir the client
      server.properties,          writes into
      world, logs
  ```

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
  version and swap the record's profile — a server also re-materialises its
  files under the `ready` gate and regenerates its properties schema; an
  instance pays at the next launch). Both directions work; a downgrade must
  be allowed explicitly, and the direction is judged by position in the
  flavor's own newest-first catalogue, not by parsing version strings.
  Servers are fully provisioned at create so `start` is an immediate spawn;
  instances are records at create and pay at launch.

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

- **`main.rs`** — bootstrap only: clap parsing (`serve`, the default, or `ping`),
  logging init (a rotated file for the long-lived daemon; stderr for `ping`), and
  dispatch.
- **`server.rs`** — the serve loop: `bind` the endpoint, then `accept` connections,
  rejecting any peer that is not `authorized()`. Each connection gets an id and an
  outbound mpsc channel drained by a writer task, so a streaming channel
  (`events.subscribe`) is an ordinary handler that pushes onto that channel. The
  loop runs under `tokio::select!` against a stop request (`daemon.stop`) and an OS
  signal (SIGTERM / Ctrl-C).
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
    - **`managers.rs`** — `DownloadManager`, `JavaInstallManager`,
      `ServerCreateManager`, and `InstanceLaunchManager`: the worker-thread
      pattern that lets `download.start` / `java.install` / `server.create` /
      `instance.launch` answer immediately while the blocking engine work runs
      off-thread, publishing progress/done/error events through the hub. The
      launch manager hands the prepared `LaunchPlan` to the supervisor under a
      deterministic process id (`server-<id>` / `instance-<id>`), so every
      channel can find a server's process without bookkeeping.
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
- **`services.rs`** — the single wire-in point: `make_router()` registers every
  channel with one `on.handle::<C>(…)` apiece. Today: `health.ping`, `app.info`,
  `daemon.status|stop` (stop takes `stop_processes`; without it supervised
  processes keep running), `config.get|set|list` (the reserved `home`/`autostart` keys
  routed to the path pointer and login registration), `cache.info|list|clear`,
  `java.releases|list|install|uninstall`, `download.start`,
  `account.login.begin|login.complete`, `account.list|switch|remove` (`switch`
  picks the default account launches use; `list` reports it),
  `process.start|stop|list|status|logs`, `events.subscribe`,
  `server.flavors|versions|resolve`,
  `server.create|update|list|status|remove|start|stop|logs|command` (create
  requires the caller to assert EULA acceptance; update refuses a running or
  still-creating server and, without `allow_downgrade`, a downgrade — a
  front-end updates a running server by explicitly stopping and restarting it
  around the job, the CLI's confirmed stop-update-start;
  start/stop/status/logs are thin over
  the supervisor, merging the stored record with live process state; command
  relays one console command over the running server's rcon channel),
  `server.config.get|set|list` (the reserved `memory`/`jvm-args` keys on the
  record plus any `server.properties` key, bar the hestia-managed ports/rcon
  ones), and the `instance.*` counterparts:
  `flavors|versions|resolve|create|update|list|remove`, plus
  `instance.launch|stop|logs` (`logs` is thin over the supervisor, like the
  server's) and `instance.config.get|set|list` (`memory`/`jvm-args` only).
- **`autostart.rs`** — registers/removes the daemon as a login-time service per
  platform, driven by the `config` service when the reserved `autostart` key is
  set (`is_enabled()` / `set()`).

> **No Service-class-per-prefix.** Unlike the historical C++ tree (which had one
> `Service` object per channel-prefix), the Rust daemon wires every channel in the
> flat `make_router()`. A handler is a closure, not a class; the registry is the
> single list.

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

## Front-ends: CLI, desktop, tray

### CLI (`cli`) — hestia

A thin client over the daemon, built on clap's derive API. `main.rs` defines a
`Command` enum — `play`, `account` (alias `auth`), `java`, `server`, `instance`,
`cache`, `config`, `daemon` — each a module under `commands/` exposing a
`Subcommand` enum and a `run()`. Global flags (`--verbose`/`--quiet`/`--home`)
sit on the root; `--home` is exported as `$HESTIA_HOME` and only takes effect
when this invocation auto-spawns the daemon (a running daemon keeps its own
directory). `commands/connect()` auto-spawns via the client SDK;
`connect_running()` requires an existing daemon.

The command grammar is noun-verb (`hestia server start`) with one deliberate
exception: `hestia play [instance]`, the launcher's single most common action,
which picks interactively when several instances exist. Anything a `create`
needs but wasn't given is asked for interactively (flavor/version pickers, the
EULA confirm) — on a terminal the picker *is* the browser; piped invocations
error with the flag to pass, so scripts stay explicit. `versions`/`flavors`
(not "available") name what they list, `ls`/`rm` alias every list/remove, and
verbs stay aligned with the wire channels (`remove`, not `delete`).

**Presentation layer (`ui/`).** Commands **never print directly** — they build a
`View` (`Line`, `Note`, `Detail`, `Table`) and hand it to `ui::show`, which owns
all output. On a terminal it renders with **ratatui** (interactive select,
scrollable pager for long tables, live install/download progress, the attach
console — live output above an input line); piped or redirected it degrades to
plain text so output stays scriptable. `select`, `prompt`, `Spinner`,
`InstallReporter`, `console`, and `human_bytes` round out the module.
This is the seam for the planned TUI: bare `hestia` (no subcommand) currently
prints help, but the intended end-state is a full-screen TUI driving the same
`View`s (à la the claude/codex model — a bare invocation is interactive, a
subcommand is scriptable).

### Desktop (`desktop`) — hestia-desktop

A Tauri v2 shell hosting the React frontend in the root `frontend/`. **Today it is
the stock Tauri template** (a `greet` command in `lib.rs`) — the shell is
scaffolded but not yet wired to the daemon over the client SDK. The design rule,
once wired, is the same one-way boundary as the CLI: the shell owns windows and
IPC, and reaches launcher logic only through `client` (never by linking `engine`).
See [contributing.md](contributing.md) for the intended `#[tauri::command]` recipe.

### Tray (`tray`)

A resident system-tray helper (daemon status + a start-at-login toggle). A
single-file placeholder; not yet ported.

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
signed-in account); in-place version updates for both (downgrades gated
behind an explicit confirmation); and the CLI front-end over all of it.

**Pending:** natives-classifier extraction for pre-1.19 clients (the resolver
skips legacy `natives-<os>` classifier libraries, so old versions launch
without their LWJGL natives) and the legacy (virtual) asset layout; wiring the
desktop shell to the daemon; and a functional tray.

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
  assembly (classpath, placeholder substitution) is unit-tested in
  `minecraft/launch.rs`.
- `crates/daemon/tests/e2e.rs` — a client-to-daemon round trip over a real socket.

Run the fast core with `cargo build -p cli -p daemon`, then
`cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.

## Recording a decision

When a non-trivial architectural choice is made, capture *what* changed and *why*
here, next to the structure it explains, so this file stays the single source of
truth rather than letting the reasoning drift into commit messages or chat logs.
