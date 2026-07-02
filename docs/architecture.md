# Architecture

This is the reference for Hestia: what exists today, where it lives, and the
reasoning behind the structure.

> **Status:** early development (`v0.0.1`). The frontend skeletons, build
> system, logging, and config store are in place — including the desktop CEF
> shell (process model, IPC bridge, embedded frontend). Launcher functionality
> is not implemented yet.

> **Daemon architecture.** Hestia runs as a daemon + thin-client model:
> `hestiad` owns the IPC endpoint, a request router, process supervision, and
> autostart; frontends are clients that drive it over a local socket (a Unix
> domain socket on POSIX, a named pipe on Windows). `libs/core` was split into
> **`libs/shared`** (IPC transport, client SDK, app identity, logging — linked
> by the daemon *and* every client) and **`libs/engine`** (config store,
> greeting, launcher logic — daemon-internal). The CLI and tray already reach
> the engine only over the socket; the desktop app still links it directly (a
> transitional exception being retired — see the target graph).

## One daemon, many frontends

Hestia is a single domain core — `hestia_engine`, owned by the daemon
(`hestiad`) — driven by several frontends, each a thin client over the socket:

| Frontend | Target           | Binary          | Stack            |
|----------|------------------|-----------------|------------------|
| Desktop  | `hestia_desktop` | `HestiaLauncher`| CEF + React/Vite |
| CLI      | `hestia_cli`     | `hestia`        | CLI11            |
| Tray     | `hestia_tray`    | `tray`          | GDBus SNI / native |

The desktop app is a separate `main()` — a thin Chromium Embedded Framework (CEF)
shell hosting a React frontend. The tray is a resident system-tray helper showing
daemon status. Each talks to the core over IPC.

The daemon (`hestiad`) and tray (`tray`) are the resident core and are
**always built**. The front-ends are opt-out via `BUILD_DESKTOP` and `BUILD_CLI`
(both `ON` by default). Skipping the desktop avoids the heaviest cost: CEF (~1 GB)
is fetched at configure time only when `BUILD_DESKTOP` is on, and the desktop
frontend must be built first (see [Build & dependency conventions](#build--dependency-conventions)).

## Target graph

```
libs/shared  → IPC transport + protocol, client SDK, shared identity (app_info.h),
               logging; linked by the daemon AND every client; zero UI dependencies
libs/engine  → launcher engine (config, greeting, launcher logic); daemon-internal
apps/cli     → CLI11 commands + thin main(); depends on shared
apps/desktop → CEF shell + embedded React frontend; depends on shared (+ engine, transitional)
apps/tray    → resident system-tray helper; depends on shared ONLY
apps/daemon  → hestiad; IPC router, supervision, autostart; the only target that links engine
```

The dependency arrows are one-way and **enforced by the build, not by
discipline**. `shared` is the common base every target links; `engine` is
daemon-only, so a front-end physically cannot reach launcher logic except over
the socket:

```
shared ◄──── cli

engine ◄──── daemon   (no front-end links engine)
```

> **Transitional:** `apps/desktop` still links `hestia_engine` directly. The
> CLI and tray already reach it only over the socket — `hestia greet`,
> `config`, and `autostart` all round-trip to the daemon via the client SDK. The
> end-state is engine = daemon-only, with every front-end reaching it over IPC.

The desktop launcher follows the same one-way rule: `apps/desktop` (namespace
`desktop::`) depends on `hestia_shared` and never the reverse. Its CEF shell knows
about windows, schemes, and IPC; it contains **no launcher logic** — that lives
in the engine and is reached over the IPC bridge. CEF's build flags are confined
to the `apps/desktop` subdirectory so the shared library and the CLI never inherit them.

## Directory layout

```
hestia-cpp/
├── CMakeLists.txt              top-level: standard, output dirs, APP_* identity, subdirectories
├── cmake/                      DownloadCEF, CMakeRC (resource compiler), PruneLocales
├── third_party/               vendored C++ deps as git submodules; cef/ fetched at configure (gitignored)
├── libs/
│   ├── shared/                hestia_shared — IPC + client SDK + identity + logging
│   │   ├── include/hestia/    PUBLIC headers (logging.h, app_info.h, ipc/*, client/*)
│   │   │                      app_info.h is GENERATED from app_info.h.in (shared identity)
│   │   └── src/               implementations (transport, protocol, client, logging)
│   └── engine/                hestia_engine — launcher engine (daemon-internal)
│       ├── include/hestia/engine/  PUBLIC headers, one folder per domain:
│       │                      engine.h (aggregate root), config/, download/,
│       │                      greeting/, support/ (cross-domain helpers)
│       └── src/               implementations, mirroring include/'s domain folders
├── apps/
│   ├── cli/                   hestia_cli — CLI11 commands + main()
│   ├── daemon/               hestia_daemon (hestiad) — bootstrap main.cc over
│   │                          src/{runtime,process,downloads,platform,services}/
│   ├── tray/                 hestia_tray — resident system-tray helper (per-platform backends)
│   └── desktop/               hestia_desktop — CEF launcher (see "Desktop launcher" below)
│       ├── frontend/          Vite + React + TS app (built with Bun) → dist/ embedded
│       └── src/core/          the CEF shell; src/features/ the IPC feature modules
└── docs/
    ├── architecture.md        this file
    ├── contributing.md        conventions + how-to recipes
    └── packaging.md           release formats, components, CEF layout, CI caching
```

### Tech stack

- **C++20**, **CMake** (≥ 3.21), built with **Ninja**.
- [spdlog](https://github.com/gabime/spdlog) + [fmt](https://github.com/fmtlib/fmt) — logging and formatting.
- [CLI11](https://github.com/CLIUtils/CLI11) — command-line parsing.
- [cpr](https://github.com/libcpr/cpr) — HTTP client for the engine's downloader
  (builds its bundled curl, fetched at configure time).
- [CEF](https://bitbucket.org/chromiumembedded/cef) — Chromium Embedded Framework (desktop).
- [React](https://react.dev/) + [Vite](https://vitejs.dev/), built with [Bun](https://bun.sh/) — desktop frontend.

The C++ libraries are vendored as git submodules under `third_party/` and added
with `add_subdirectory`; their tests/examples/docs and install rules are turned
off in `third_party/CMakeLists.txt`. **CEF is the exception**: it is a ~1 GB
prebuilt binary distribution fetched at configure time by `cmake/DownloadCEF.cmake`
into `third_party/cef/` (gitignored, SHA-verified, pinned via `CEF_VERSION`). The
frontend's `dist/` tree is compiled into the binary by **CMakeRC**
(`cmake/CMakeRC.cmake`) as an in-memory virtual filesystem.

### Namespaces

| Namespace              | Home           | Contents                                   |
|------------------------|----------------|--------------------------------------------|
| `hestia`               | `libs/shared`  | cross-cutting (`init_logging`, `LogLevel`) |
| `hestia::ipc`          | `libs/shared`  | transport, endpoint, protocol envelope     |
| `hestia::client`       | `libs/shared`  | typed client SDK (`Client`)                |
| `hestia::engine`       | `libs/engine`  | `Engine` aggregate root + `ConfigStore`    |
| `hestia::config`       | `libs/engine`  | data-dir resolution + `Config` store       |
| `hestia::greeting`     | `libs/engine`  | the demo `greet()` function                |
| `hestia::cli`          | `apps/cli`     | command framework + commands               |
| `desktop::core`†       | `apps/desktop` | CEF shell — app/browser/window/scheme      |
| `desktop::ipc`         | `apps/desktop` | the JS⇄C++ bridge (router + registry)      |
| `desktop::features`    | `apps/desktop` | IPC feature modules (app, window, …)       |

† the shell sub-namespaces are `desktop::app`, `desktop::browser`,
`desktop::common`, `desktop::window`. The desktop is deliberately **not** under
`hestia::` — it is a UI shell over the engine, not part of it. Shared identity
macros (`APP_NAME`, `APP_VERSION`, …) come from `<hestia/app_info.h>` and are used
by every frontend.

## Shared library (`libs/shared`)

UI-free, cross-cutting code linked by the daemon **and** every client. It carries
the one public boundary between them — the socket — and nothing launcher-specific.

- **`logging`** — `init_logging(LogLevel)` configures the process-wide spdlog
  logger once at startup. `LogLevel` is Hestia's own enum so callers don't depend
  on spdlog's; `logging.cc` maps it across.
- **`ipc`** — the platform transport (`transport_posix.cc` / `transport_windows.cc`),
  endpoint resolution, and the JSON protocol envelope. It also carries the wire
  **vocabulary** for domains that cross the socket — `process.h`/`process_codec.h`
  and `download.h`/`download_codec.h` pair the domain types with their one
  JSON codec, so the daemon and every client (de)serialize through the same
  definitions and cannot drift.
- **`client`** — the typed client SDK (`hestia::client::Client`) front-ends use to
  drive the daemon.
- **identity** — `app_info.h` is generated from `app_info.h.in` and exposed on the
  **public** interface, so every target shares one source of truth (name, id,
  vendor, version, channel).

`shared` links spdlog and fmt **privately**; `nlohmann_json` is **public** because
it appears in the protocol envelope headers.

## Engine library (`libs/engine`)

The launcher engine — daemon-internal domain logic, the equivalent of Tailscale's
`LocalBackend`. Front-ends reach it over the socket, not by linking it (with the
transitional exception noted in the target graph).

**`hestia::engine::Engine`** (`engine.h`) is the aggregate root: the daemon
constructs exactly one and threads it through every request handler (see
`HandlerContext`). It resolves the data directory once at startup and owns each
domain subsystem as a member, exposed through a getter (`engine.config()`). This
is the single growth point for launcher logic — adding a domain (instances,
accounts, versions, …) is a module class constructed in `Engine`'s initializer
list plus a getter, with no change to the daemon's wiring. `set_data_home()`
re-resolves the data directory and repoints every subsystem so a `config.set-home`
takes effect on the running daemon, not just the next start.

Headers live under `include/hestia/engine/<domain>/`, one folder per domain, with
`src/` mirroring the same layout — a new domain gets a folder, not a longer flat
list. The subsystems behind the aggregate today:

- **`ConfigStore`** (`config/config_store.h`) — a thread-safe live view of the
  flat `key=value` config file. Reads/writes are serialized for concurrent clients
  and every `set()` is persisted immediately; `reload()` repoints it when the data
  directory changes. Built on the lower-level `config` module below.
- **`config`** (`config/config.h`) — data-directory resolution and the flat
  `key=value` `Config` store. Resolution precedence (`data_home`): `--home`
  override → `$HESTIA_HOME` → a persisted pointer file under the anchor dir
  (`~/.hestia` or `%APPDATA%\Hestia`) → the platform default. `Config::load` /
  `get` / `set` / `save` operate on the file at `config_path()`. A missing file is
  an empty config, not an error.
- **`Downloader`** (`download/downloader.h`) — streams a URL to disk through a
  `.part` temp file (via cpr), hashing incrementally when a checksum is given and
  renaming into place only on success. Stateless, so it hangs off no aggregate —
  the daemon's download manager constructs one per download. Its checksum
  vocabulary (`ipc::Checksum`, `ipc::HashAlgorithm`) comes from
  `<hestia/ipc/download.h>`, the same types the wire uses.
- **`support/checksum`** — native incremental SHA-1/SHA-256 (`Hasher`), so a
  download is verified as it streams rather than re-read afterwards.
- **`greeting`** (`greeting/greeting.h`) — `greet(name)`; a placeholder
  exercising the engine→frontend seam.

`engine` links fmt and cpr **privately** — implementation details that do not leak
through its public headers. `hestia_shared` is linked **publicly**: the engine's
headers expose shared download types, and it resolves paths through
`hestia::paths`.

## Daemon (`apps/daemon`)

`hestiad` is the resident core: it owns the IPC endpoint, routes requests to
services, supervises launched processes, and manages autostart. It is the only
target that links `hestia_engine` directly.

`src/` is split into a thin bootstrap plus one folder per concern; local includes
are subdir-qualified (`"runtime/router.h"`):

- **`main.cc`** — bootstrap only: CLI11 parsing, logging init, `hestiad ping`,
  and a delegation to `runtime::run_daemon()`.
- **`runtime/`** — the serving machinery. `server.{h,cc}` binds the endpoint,
  builds the `Runtime`, registers the services, and serves connections until
  signalled; `router.{h,cc}` maps a channel string to a handler
  (`router.on("config.get", …)`); `event_hub.h` fans daemon events out to
  subscribed connections.
  - **`Runtime`** (`runtime.h`) — the one place the daemon's long-lived
    collaborators live: the engine, the event hub, the download manager, and the
    process supervisor, constructed in dependency order (the hub before anything
    that publishes into it, so reverse-order destruction tears the publishers
    down first). Adding a subsystem is a member plus an accessor here — the serve
    loop and every existing service are untouched.
  - **`HandlerContext`** (`handler_context.h`) — what every handler receives:
    `{Runtime &runtime, connection, peer}`. Handlers reach collaborators through
    accessors (`ctx.runtime.engine()`, `ctx.runtime.supervisor()`, …); the
    per-request connection is what lets streaming channels (`events.subscribe`)
    be ordinary handlers.
- **`services/`** — one file per channel-prefix, wired in once via
  `register_all_services()` in `services.h`. Today: `health` (`health.ping`),
  `app` (`app.info`, `app.greet`), `config` (`config.get|set|home|set-home`),
  `process` (`process.start|stop|list|status|logs`), `autostart`
  (`autostart.enable|disable|status`), `downloads` (`download.start`), and
  `events` (`events.subscribe`, a streaming channel that pushes to the calling
  connection).
- **`downloads/`** (`download_manager.{h,cc}`) — runs each download on a worker
  thread so `download.start` answers immediately; progress and the terminal
  outcome are published through the event hub as `download.progress`,
  `download.done`, and `download.error` events (throttled progress, filtered by
  the download's id like process events).
- **`process/`** (`process_supervisor`, `process_table`, `process_spawner`,
  `liveness_probe`, `log_streamer`, `restart_policy`) — launches Minecraft (and
  other) processes as children of the daemon, tracks them in a process table,
  probes liveness, streams their logs, and applies a restart policy. Reaping the
  children yields their exit codes.
- **`platform/`** (`autostart.{h,cc}`, `win_util.h`) — registers/removes the
  daemon as a login-time service per platform, driven over the `autostart.*`
  channels, plus Windows-specific helpers.

The daemon links `hestia_shared`, `hestia_engine`, CLI11, nlohmann_json, and
spdlog — no UI dependencies.

## Tray (`apps/tray`)

The `tray` binary (`hestia_tray` target) is a resident system-tray helper that surfaces daemon status and a
one-click toggle for starting the daemon at login. It depends on `hestia_shared`
**only** (no engine link — it talks over the socket) and has a per-platform
backend: `backend_linux.cc` (GDBus StatusNotifierItem), `backend_windows.cc`
(Shell_NotifyIcon), and `backend_macos.mm` (Cocoa). A `single_instance` guard
allows only one tray per session.

## CLI command system (`apps/cli`)

Commands are objects implementing the `Command` interface
(`command.h`): each `register_command(parent, ctx)` attaches its subcommand,
options, and callback onto a parent `CLI::App`. Because the parent can be the
root app *or* another command's app, commands **nest to any depth**.

- **`Command`** — a leaf unit of functionality (e.g. `greet`).
- **`CommandGroup`** — a `Command` that holds children and registers them onto
  its own subcommand app (e.g. `config get|set|home|set-home`). The same
  mechanism recurses.
- **`AppContext`** — parsed `GlobalOptions` (`--verbose`/`--quiet`/`--home`) plus
  the collected `exit_code`, passed by reference to every command.
- **`make_commands()`** (`registry.cc`) — the single place a top-level command is
  wired in.

`cli.cc::run()` builds the root `CLI::App`, binds the global flags, configures
logging via a parse-complete callback (so verbosity is set before any command
callback runs), registers `make_commands()`, parses, and returns the context's
exit code. No subcommand → print help.

## Desktop launcher (`apps/desktop`)

A thin CEF shell hosting the React frontend. The design rule: the shell
(`desktop::`) owns windows, schemes, and IPC; **all launcher logic stays in
`hestia_engine`** and is reached over the bridge. Two halves: `src/core/` is the
reusable shell you rarely touch; `src/features/` is where day-to-day work happens.

### Process model (`src/core/app`)

CEF runs the browser, renderer, GPU, and utility roles as **sub-processes of the
same executable**, distinguished by the `--type` switch. `main_util.cc` maps that
to a `ProcessType` and `CreateApp()` returns the right `CefApp`:

- `AppBase` — registers the custom scheme in *every* process (schemes must match
  across all of them).
- `BrowserApp` — `OnContextInitialized()` is the wiring hub: init settings,
  register the scheme handler, init the IPC router, register features, then create
  the browser view + frameless window.
- `RendererApp` — hosts the renderer side of the message router. It creates the
  router in **`OnWebKitInitialized()`** (not later) — that is when the router
  registers the native `window.cefQuery` binding; creating it in `OnContextCreated`
  misses that window and the bridge silently never appears. It also turns
  native→JS event messages into DOM `CustomEvent`s.
- `OtherApp` — gpu/utility processes (scheme registration only).

> **Linux zygote.** On Linux the zygote process forks into the other roles and
> the fork inherits this process's `CefApp`. Since the eventual role is unknown at
> fork time, the zygote is given the **renderer** app — otherwise forked renderers
> get no render-process handler and `cefQuery` is never injected. See
> `GetProcessType()` in `main_util.cc`.

### IPC bridge (`src/core/ipc`)

Built on CEF's message router (`window.cefQuery`). Wire format is
`{ "channel": "...", "payload": <any> }`; responses are JSON. A global `Registry`
maps a channel to a `Handler`; a scoped `Actions` registrar prefixes channels by
feature name (`Actions("app")("info", …)` → channel `app.info`). Handlers answer
synchronously, or hold the copyable `Response` and answer later from any thread.
Push native→JS events with `ipc::Emit(browser, "channel", value)` — they arrive in
the page as `window.addEventListener(channel, e => e.detail)`. The browser- and
renderer-side routers share one `RouterConfig()` so their function names match.

### Window & scheme (`src/core/window`, `src/core/common`)

`WindowDelegate` hosts the browser view in a **frameless** top-level window — the
React app draws its own title bar and drives minimize/maximize/close over the
`window.*` IPC channels. `app_scheme.cc` registers a standard, secure, fetch-
enabled scheme (`hestia://app/`) and serves the embedded `dist/` tree from the
CMakeRC virtual filesystem, with MIME detection and an SPA fallback to
`index.html`. `app_settings.cc` decides the startup URL.

### Embedded vs. dev server

`GetStartupURL()` returns the dev-server URL when set, else the embedded scheme.
The dev path is **Debug-only and compiled out of Release** (`#if !defined(NDEBUG)`):
the `--dev-url` switch, the `APP_DEV_SERVER_URL` env var, and the compile-time
default are all ignored in Release, so production can only load embedded assets.
This is what enables frontend hot-reload in development (Vite HMR) while keeping
the shipped build self-contained.

### Feature modules (`src/features`)

Launcher functionality is added as a **feature module**, never by editing the
shell. A `Feature` declares its channel-prefix `Name()` and registers its actions;
`feature_registry.cc` is the one list where features are wired in. The example
`AppFeature` (`app.info`, `app.ping`, `app.greet`) proves the bridge reaches the
engine — `app.greet` calls `hestia::greeting::greet()` in the engine. `WindowFeature`
drives the frameless window.

### Sandbox & size (Release)

The CEF sandbox is **on in Release, off in Debug** (`USE_SANDBOX`, defaulted by
build type). On Linux the sandbox lives inside `libcef.so`; the `chrome-sandbox`
helper must be SUID root once per build location (CMake prints the command).
Release post-build steps strip `libcef.so` (~1.4 GB → ~200 MB) and prune unused
locales to `en-US.pak`.

## Build & dependency conventions

- **One configure builds everything by default**, gated by the `BUILD_DESKTOP`
  and `BUILD_CLI` toggles (both `ON`; the daemon and tray are always built). With
  the desktop on, the configure fetches CEF (~1 GB, first run only) and requires a
  built `frontend/dist`, so **build the frontend before configuring** (CMakeRC globs
  `dist/` at configure time; a missing `dist/` is a hard `FATAL_ERROR`):
  `(cd apps/desktop/frontend && bun run build)` → `cmake … -B build` →
  `cmake --build`. A `-DBUILD_DESKTOP=OFF` configure skips CEF entirely for a fast
  daemon/CLI loop.
- Each library/app sets `-Wall -Wextra -Wpedantic` (GNU/Clang) or `/W4` (MSVC).
  CEF discovery is isolated inside `apps/desktop/` so its compiler/linker flags
  never reach the shared library, the engine, or the CLI.
- The root `CMakeLists.txt` defines the `APP_*` identity (name, id, vendor,
  channel; version from `project(... VERSION)`). `hestia_shared` generates
  `<hestia/app_info.h>` from these and exposes it on its **public** interface, so
  every frontend shares one source of truth — the CLI's `--version`, the desktop's
  `app.info`, etc. all read the same `APP_VERSION`.
- Dependencies that don't appear in a target's public headers are linked
  `PRIVATE` (e.g. shared's spdlog/fmt). They still propagate as transitive link
  deps to the final executable.
- Build artifacts land in `build/<config>/` (e.g. `build/Release/`); the desktop
  binary and its CEF runtime files are copied alongside it.

## See also

- [contributing.md](contributing.md): step-by-step recipes for adding views,
  layouts, components, overlays, and CLI commands.
