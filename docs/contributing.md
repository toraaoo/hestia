# Contributing — conventions & recipes

Practical, copy-and-adapt guides for extending Hestia. Read
[architecture.md](architecture.md) first for the lay of the land.

## Conventions

A few rules hold everywhere; the recipes below assume them.

- **`rustfmt` + `clippy -D warnings` stay clean.** No exceptions; CI enforces both,
  plus `cargo-deny`.
- **Wire-in is one line.** Each kind of thing has exactly one place it is added — a
  `Command` enum variant (CLI), a `handle::<C>` in `make_router()` (daemon), a
  facade accessor (client). Adding a feature should not touch the serve loop or the
  transport.
- **One thing per file / module.** A CLI domain is its own module under
  `commands/`; an engine domain is its own module under `engine/src/`.
- **Single-word module names** (`config`, not `config_store`). Follow Rust naming
  throughout.
- **Errors:** `thiserror` enums in library crates, mapped to an `ipc::errors` code
  at the daemon's service boundary (via `ServiceError`); `anyhow` at binary edges
  and for multi-step engine operations. Don't panic on recoverable errors.
- **Logging:** `tracing` at appropriate levels. **Never log tokens or secrets.**
- **Identity comes from one place:** `common::app` (`NAME`, `VERSION`, `ID`,
  `VENDOR`, `CHANNEL`). Don't hard-code the product name or version.
- **Immutable/at-the-edge validation:** validate external data where it enters
  (payloads decode through the contract; the config schema rejects unknown keys).

## The wire-in map

Most features touch the same five seams, one line each. The `config` channels are
the shipped end-to-end reference (`hestia config get home` round-trips
`config.get` → `ConfigService` handler → `engine.config()`).

| Seam            | Where                                 | What                                                  |
|-----------------|---------------------------------------|-------------------------------------------------------|
| Wire contract   | `crates/proto/src/<domain>.rs`        | a struct + `impl Contract` (serde codec)              |
| Engine domain   | `crates/engine/src/<domain>/`         | a module hung off the `Engine` aggregate              |
| Daemon channel  | `crates/daemon/src/services.rs`       | one `on.handle::<C>(…)` in `make_router()`            |
| Client facade   | `crates/client/src/facades.rs`        | a one-liner over `Session::call::<C>()`               |
| CLI command     | `crates/cli/src/commands/<domain>.rs` | a `clap` `Subcommand` + a `run()`, wired in `main.rs` |
| Desktop command | `crates/desktop/src/` (Tauri)         | a `#[tauri::command]` in `generate_handler!`          |

---

## Add a wire contract

Contracts live in `crates/proto`, one module per domain. A call contract names its
channel once and pairs it with `Params`/`Result`; serde derive is the codec, so
both sides marshal through this single definition and cannot drift.

```rust
// crates/proto/src/instances.rs
use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct Instance {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceListResult {
    pub instances: Vec<Instance>,
}

pub struct InstanceList;
impl Contract for InstanceList {
    const CHANNEL: &'static str = "instance.list";
    type Params = Empty;
    type Result = InstanceListResult;
}
```

Add `pub mod instances;` to `crates/proto/src/lib.rs`. Use `#[serde(default)]` on
payloads so an older/newer peer that omits a field still decodes (additive fields
need no protocol bump). For a daemon→client push, implement `Topic` instead of
`Contract` — the type is its own event payload:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstanceProgressEvent {
    pub id: String,
    pub percent: u8
}
impl Topic for InstanceProgressEvent {
    const TOPIC: &'static str = "instance.progress";
}
```

If you change an existing payload's shape, update `crates/proto/tests/` (the
`wire`/`golden` encodings are pinned on purpose).

---

## Add an engine domain

Launcher/domain logic lives in `crates/engine`, daemon-internal. A subsystem hangs
off the `Engine` aggregate root (`engine.rs`), which the daemon owns and hands to
every handler. The worked example adds an `instances` store; model it on `config`.

**1. Write the subsystem** as a module under `crates/engine/src/instances/` (or a
single `instances.rs` if small). Take a path under the data dir in the
constructor, serialize access with a `Mutex` for concurrent clients, and offer a
`reload()` so a data-home change repoints it.

```rust
// crates/engine/src/instances/mod.rs
use std::path::PathBuf;
use std::sync::Mutex;

use proto::instances::Instance;

pub struct Instances {
    inner: Mutex<PathBuf>,
}

impl Instances {
    pub fn new(dir: PathBuf) -> Self {
        Instances { inner: Mutex::new(dir) }
    }

    pub fn reload(&self, dir: PathBuf) {
        *self.inner.lock().unwrap() = dir;
    }

    pub fn list(&self) -> Vec<Instance> {
        // ... scan self.inner.lock().unwrap() ...
        Vec::new()
    }
}
```

Add `mod instances;` and a `pub use` to `crates/engine/src/lib.rs`.

**2. Hang it off `Engine`** (`engine.rs`) — a field, a getter, construction in
`new()`, and a `reload()` line in `set_data_home()`. This is the *only* change to
the engine's wiring; `HandlerContext` already carries the `Engine`.

```rust
// crates/engine/src/engine.rs — inside struct Engine
instances: Instances,

// in new():
let instances = Instances::new(data_home.join("instances"));

// in set_data_home(), alongside the other reloads:
self .instances.reload(resolved.join("instances"));

// a getter:
pub fn instances(&self) -> &Instances { &self.instances }
```

A stateless helper (like `minecraft`) needs no data dir and can be constructed
without a path; a stateless *free* function needs no aggregate member at all.

---

## Add a daemon channel

One `on.handle::<C>(…)` in `make_router()` (`crates/daemon/src/services.rs`). The
registrar decodes `C::Params` (a malformed payload answers `bad_request` for you)
and encodes the returned `C::Result`; the handler reaches collaborators through
`ctx.runtime.*()` and returns a `ServiceError` for a typed failure.

```rust
// crates/daemon/src/services.rs — inside make_router()
use proto::instances::{InstanceList, InstanceListResult};

on.handle::<InstanceList, _, _ > ( | _: Empty, ctx| async move {
  Ok(InstanceListResult {
    instances: ctx.runtime.engine().instances().list(),
  })
});
```

Map engine errors to codes with `ServiceError::not_found` / `bad_request` /
`handler_error`. For a long-running operation, follow `JavaInstallManager`: kick
the blocking work onto a manager that answers immediately and publishes progress /
done / error `Topic`s through `ctx.runtime.hub()`.

---

## Add a client facade method

Facade methods are one-liners over `Session::call::<C>()` that return `proto` types
directly (`crates/client/src/facades.rs`).

```rust
pub struct Instance<'a> {
    pub(crate) session: &'a Session,
}

impl Instance<'_> {
    pub async fn list(&self) -> Result<Vec<proto::instances::Instance>, IpcError> {
        Ok(self
            .session
            .call::<proto::instances::InstanceList>(&proto::Empty {})
            .await?
            .instances)
    }
}
```

If the domain is new, add the accessor on `Client` (`crates/client/src/lib.rs`) and
export the facade type from the crate root. Use `try_call` when a `not_found`
should surface as `None`, `call_with_timeout` for a long call, and `run_job` to
block on a progress-streaming operation (see `Java::install` / `Process::run`).

---

## Add a CLI command

Commands are clap `Subcommand` enums, one module per domain under
`crates/cli/src/commands/`, dispatched from `main.rs`. Commands **never print
directly** — they build a `View` and hand it to `ui::show`.

**1. The module** (`crates/cli/src/commands/instance.rs`):

```rust
use anyhow::Result;
use clap::Subcommand;

use crate::commands::connect;
use crate::ui::{self, View};

#[derive(Subcommand)]
pub enum InstanceCmd {
    /// List instances
    List,
}

pub async fn run(cmd: InstanceCmd) -> Result<()> {
    match cmd {
        InstanceCmd::List => list().await,
    }
}

async fn list() -> Result<()> {
    let client = connect().await?;
    let instances = client.instance().list().await?;
    if instances.is_empty() {
        return ui::show(View::note("no instances yet"));
    }
    ui::show(View::table(
        "Instances",
        ["ID", "NAME"],
        instances.into_iter().map(|i| vec![i.id, i.name]).collect(),
    ))
}
```

Use `connect()` to auto-spawn the daemon, or `connect_running()` when the command
must not start it. Build `View::line` / `note` / `detail` / `table`; call
`ui::select` for an interactive pick (it errors when stdin is not a terminal, so
offer an argument as the fallback), and `ui::human_bytes` for sizes.

**2. Wire it in** `crates/cli/src/main.rs`:

```rust
#[derive(Subcommand)]
enum Command {
    // ...
    /// Minecraft instances
    Instance {
        #[command(subcommand)]
        cmd: commands::instance::InstanceCmd,
    },
}

// in dispatch():
Command::Instance { cmd } => commands::instance::run(cmd).await,
```

Add `pub mod instance;` to `crates/cli/src/commands/mod.rs`.

---

## Add a desktop command (Tauri)

> **Current state:** `crates/desktop` is the stock Tauri v2 template — a `greet`
> command in `lib.rs`, not yet wired to the daemon. The recipe below is the
> intended pattern once the shell is connected: the desktop reaches launcher logic
> only through `client` (never by linking `engine`), exactly like the CLI.

A desktop feature is a `#[tauri::command]` that calls a `client` facade, plus the
frontend code that invokes it. Keep the commands in one module (e.g. `api.rs`) and
register them in `generate_handler!`:

```rust
// crates/desktop/src/api.rs
#[tauri::command]
pub async fn instance_list() -> Result<Vec<serde_json::Value>, String> {
    let client = client::Client::connect(true).await.map_err(|e| e.to_string())?;
    let instances = client.instance().list().await.map_err(|e| e.to_string())?;
    Ok(instances.into_iter().map(|i| serde_json::json!(i)).collect())
}
```

```rust
// crates/desktop/src/lib.rs
tauri::Builder::default ()
.invoke_handler(tauri::generate_handler![api::instance_list])
// ...
```

The frontend calls it with Tauri's `invoke("instance_list")`. Wiring the shell to
the daemon (a shared client/session, event forwarding) is itself pending work —
see [architecture.md](architecture.md).

---

## Build & run

The core loop needs no webview or frontend deps:

```bash
cargo build -p cli -p daemon
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Iterate interactively with `scripts/dev.sh` — it opens a subshell with `hestia`
and `hestiad` on `PATH` (debug builds keep data in `<repo>/.hestia`), or
`scripts/dev.sh java list` for a one-shot. The daemon auto-spawns on first client
connect, so most CLI commands "just work" without starting it by hand;
`hestia daemon status|start|stop|restart` manages it explicitly.

The desktop app needs the system webview (WebKitGTK on Linux, WebView2 on Windows)
and the Bun-built frontend:

```bash
(cd frontend && bun install && bun run build)
scripts/dev.sh --desktop        # Tauri shell with frontend HMR
```

See [packaging.md](packaging.md) for installers and sidecar bundling.

## Recording a decision

When a non-trivial architectural choice is made, capture *what* changed and *why*
in [architecture.md](architecture.md) so the reference stays the single source of
truth — next to the structure it explains, not in commit messages or chat logs.
