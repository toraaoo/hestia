# Architecture

This is the reference for Hestia: what exists today, where it lives, and the
reasoning behind the structure.

> **Status:** early development (`v0.0.1`). The frontend skeletons, build
> system, logging, and config store are in place — including the desktop CEF
> shell (process model, IPC bridge, embedded frontend). Launcher functionality
> is not implemented yet.

> **Migration in progress (daemon architecture).** Hestia is moving to a
> daemon + thin-client model: `hestiad` owns process supervision and frontends
> become clients over a local socket. As part of this, `libs/core` was split
> into **`libs/shared`** (IPC transport, app identity, logging — linked by the
> daemon *and* every client) and **`libs/engine`** (config store, greeting,
> launcher logic — daemon-internal). The structural sections below (target
> graph, layout, namespaces, library reference) describe the split; the deeper
> TUI/desktop internals predate it and get their full rewrite in the migration's
> hardening phase. See [daemon-protocol.md](daemon-protocol.md) for the socket
> boundary.

## One core, three frontends

Hestia is a single domain core — `hestia_engine`, owned by the daemon — driven
three ways:

| Frontend    | Target           | Binary          | Stack                     |
|-------------|------------------|-----------------|---------------------------|
| Desktop     | `hestia_desktop` | `Hestia`        | CEF + React/Vite          |
| CLI         | `hestia_cli`     | `hestia`        | CLI11                     |
| TUI         | `hestia_tui`     | *(in `hestia`)* | FTXUI                     |

The CLI and TUI ship in **one binary**: `hestia tui` launches the interactive
terminal UI, every other subcommand is plain CLI. The desktop app is a separate
`main()` over the same core — a thin Chromium Embedded Framework (CEF) shell that
hosts a React frontend and talks to the core over an IPC bridge.

A single configure builds **all** of them; there are no `BUILD_*` toggles. The
only cost is that CEF (~1 GB) is fetched at configure time and the desktop
frontend must be built first (see [Build & dependency conventions](#build--dependency-conventions)).

## Target graph

```
libs/shared  → IPC transport + protocol, client SDK, shared identity (app_info.h),
               logging; linked by the daemon AND every client; zero UI dependencies
libs/engine  → launcher engine (config, greeting, launcher logic); daemon-internal
libs/tui     → FTXUI app; depends on shared ONLY; public surface = hestia::tui::run()
apps/cli     → CLI11 commands + thin main(); depends on shared + tui
apps/desktop → CEF shell + embedded React frontend; depends on shared
apps/daemon  → hestiad; the only target that links engine
```

The dependency arrows are one-way and **enforced by the build, not by
discipline**. `shared` is the common base every target links; `engine` is
daemon-only, so a front-end physically cannot reach launcher logic except over
the socket:

```
shared ◄──── tui
  ▲          ▲
  └─── cli ──┘

engine ◄──── daemon   (no front-end links engine)
```

> **Transitional:** `apps/desktop` still links `hestia_engine` directly. The
> CLI and TUI already reach it only over the socket (`hestia greet` round-trips
> `app.greet` to the daemon). The end-state is engine = daemon-only, with every
> front-end reaching it over IPC.

`libs/tui` physically cannot `#include` a CLI command — it does not link
`apps/cli`, so it cannot see it. `libs/tui` exposes exactly one public header,
`hestia/tui/run.h`; everything under `libs/tui/src/` is private to the library.
This is why `apps/cli` reaches the TUI through a single symbol and never touches
its internals.

The desktop launcher follows the same one-way rule: `apps/desktop` (namespace
`desktop::`) depends on `hestia_shared` and never the reverse. Its CEF shell knows
about windows, schemes, and IPC; it contains **no launcher logic** — that lives
in the engine and is reached over the IPC bridge. CEF's build flags are confined
to the `apps/desktop` subdirectory so the shared library, the CLI, and the TUI
never inherit them.

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
│   ├── engine/                hestia_engine — launcher engine (daemon-internal)
│   │   ├── include/hestia/    PUBLIC headers (config.h, greeting.h)
│   │   └── src/               implementations
│   └── tui/                   hestia_tui — terminal UI library
│       ├── include/hestia/tui/run.h   the ONLY public header
│       └── src/               private internals (see "TUI internals" below)
├── apps/
│   ├── cli/                   hestia_cli — CLI11 commands + main()
│   └── desktop/               hestia_desktop — CEF launcher (see "Desktop launcher" below)
│       ├── frontend/          Vite + React + TS app (built with Bun) → dist/ embedded
│       └── src/core/          the CEF shell; src/features/ the IPC feature modules
└── docs/
    ├── architecture.md        this file
    └── contributing.md        conventions + how-to recipes
```

### Tech stack

- **C++20**, **CMake** (≥ 3.21), built with **Ninja**.
- [spdlog](https://github.com/gabime/spdlog) + [fmt](https://github.com/fmtlib/fmt) — logging and formatting.
- [CLI11](https://github.com/CLIUtils/CLI11) — command-line parsing.
- [FTXUI](https://github.com/ArthurSonzogni/FTXUI) — terminal user interface.
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

| Namespace               | Home            | Contents                              |
|-------------------------|-----------------|---------------------------------------|
| `hestia`                | `libs/shared`   | cross-cutting (`init_logging`, `LogLevel`) |
| `hestia::ipc`            | `libs/shared`   | transport, endpoint, protocol envelope |
| `hestia::client`         | `libs/shared`   | typed client SDK (`Client`)           |
| `hestia::config`        | `libs/engine`   | data-dir resolution + `Config` store  |
| `hestia::greeting`      | `libs/engine`   | the demo `greet()` function           |
| `hestia::cli`           | `apps/cli`      | command framework + commands          |
| `hestia::tui`           | `libs/tui`      | the terminal UI (everything)          |
| `hestia::tui::keys`     | `libs/tui`      | global key predicates                 |
| `hestia::tui::layout`   | `libs/tui`      | layout id constants                   |
| `hestia::tui::overlay`  | `libs/tui`      | overlay id constants                  |
| `desktop::core`†        | `apps/desktop`  | CEF shell — app/browser/window/scheme |
| `desktop::ipc`          | `apps/desktop`  | the JS⇄C++ bridge (router + registry) |
| `desktop::features`     | `apps/desktop`  | IPC feature modules (app, window, …)  |

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
  endpoint resolution, and the JSON protocol envelope. See
  [daemon-protocol.md](daemon-protocol.md) for the wire format.
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
transitional exception noted in the target graph). Two modules today:

- **`config`** — data-directory resolution and a flat `key=value` store.
  Resolution precedence (`data_home`): `--home` override → `$HESTIA_HOME` → a
  persisted pointer file under the anchor dir (`~/.hestia` or `%APPDATA%\Hestia`)
  → the platform default. `Config::load` / `get` / `set` / `save` operate on the
  file at `config_path()`. A missing file is an empty config, not an error.
- **`greeting`** — `greet(name)`; a placeholder exercising the engine→frontend seam.

`engine` links fmt **privately** — it is an implementation detail and does not leak
through its public headers.

## TUI internals (`libs/tui/src`)

The TUI is designed component-first, with a deliberate caveat: **FTXUI is not
reactive.** You build the component tree **once**; a render loop re-runs each
`Renderer`'s lambda **every frame**, rebuilding the element tree from whatever
your plain C++ state variables currently hold. There is no `useState`/re-render
diff. Two trees coexist that a framework like React would fuse into one:

- the **Component tree** — events/focus/handlers, built once;
- the **Element tree** — `Render()` output, rebuilt per frame.

Holding that distinction in mind prevents most FTXUI confusion. Most React
intuition still transfers — a `Component` is a reusable node, a `View` is a
page/route, the `Navigator` is the router, `AppContext` is `<Context.Provider>`,
and `on_enter`/`on_exit` are mount/unmount. The one thing that does *not* map is
reactivity: no `useState`, no effect cleanup, no dependency arrays. You own the
state and the loop re-reads it.

### Subsystems

```
libs/tui/src/
  tui_app.cc          run(): builds the shell, owns everything, runs the loop
  app_context.h       AppContext — services/state passed "props-down" to views
  navigation/
    route.h           RouteId (string id for a view)
    view.h            View — abstract route-level screen
    navigator.{h,cc}  the router: active selection, lifecycle hooks, overlays
    view_registry.cc  make_views() — the one place a view is wired in
  layout/
    layout.h          Layout interface + LayoutSlots + layout ids
    layout_registry.{h,cc}  make_layouts() — id → Layout, never fails silently
    layouts/          sidebar (default), fullscreen, centered
    header_bar / status_bar / sidebar   reusable slot builders
  components/         dumb presentational pieces: panel, button, key_hint
  input/
    keymap.h          key predicates (quit = 'q', cancel = Esc)
    global_keys.{h,cc}  with_global_keys(): app-wide bindings, modal-aware
  overlays/           modal layers: confirm_quit
  theme/theme.h       semantic styling roles (terminal-honoring, no palette)
  views/              the actual screens: home, about
```

### Key abstractions

- **View** (`navigation/view.h`) — a route-level screen. Owns its component
  subtree (built once in `build(ctx)`), declares its `id()`, `title()`, and which
  `layout()` arranges it (default `Sidebar`). Optional `on_enter`/`on_exit`
  lifecycle hooks fire on navigation. Analogous to a React page component:
  composed *from* `components/`, never the reverse.
- **Navigator** (`navigation/navigator.h`) — the router. Owns the active-route
  index (shared by reference with the sidebar `Menu` and the content `Tab` via a
  single `int`, so moving the menu swaps the view) and the current overlay id.
  `tick()` runs once per frame to detect a menu-driven selection change and fire
  the view lifecycle hooks.
- **Layout** (`layout/layout.h`) — a **pure** `Element`-level arranger:
  `arrange(LayoutSlots)` decides placement only. Interactive components stay
  built-once in the shell, so swapping a layout never rebuilds the component tree
  and focus/event routing stays centralised. Lookup falls back to `Sidebar` with
  a warning on an unknown id — never undefined behaviour.
- **Component** (`components/`) — dumb, presentational, props-in via factory
  args, no navigation or app logic. Reused across views.
- **Overlay** (`overlays/`) — a transient modal layer stacked above the active
  view. While one is open, `with_global_keys` steps aside and the overlay owns
  all input.
- **Theme** (`theme/theme.h`) — styling expressed as semantic *roles*
  (`brand`, `emphasis`, `muted`, `selected`, `normal`), each an `ftxui::Decorator`
  built only from terminal-honoring primitives (`Color::Default`, bold/dim/
  inverted). **No private palette:** hierarchy comes from attributes, so the UI
  inherits the user's terminal colors. Components and layouts never hard-code a
  color — they pull from a role.

### Runtime flow (`run()` in `tui_app.cc`)

1. Create a fullscreen `ScreenInteractive` and a `Theme`.
2. `make_views()` builds and owns the views; the `Navigator` routes over
   non-owning pointers.
3. Populate `AppContext` with the theme, navigator, and the quit closures
   (`request_quit` opens the confirm-quit overlay; `exit_app` is the loop-exit).
4. Build **every** interactive component once: a `Container::Tab` of per-view
   panels (keyed on the navigator's shared selection int), the sidebar menu, and
   the overlay. A two-entry `Container::Tab` routes focus to exactly one layer —
   main pane or overlay.
5. A single `Renderer` lambda runs per frame: `nav.tick()`, route focus, read the
   active view's `layout()`, fill the `LayoutSlots`, and return
   `layouts.get(id).arrange(slots)`.
6. `with_global_keys` wraps the renderer; `nav.start()` fires the first
   `on_enter`; `screen.Loop(root)` runs.

State is plain variables — no reactive store. State ownership is **binary**:
shared state lives in `AppContext`; everything else is a component-local member
field. Nothing ad-hoc-global is threaded through. A reactive global store is
deliberately deferred; if one is ever needed it would be hand-built, not free.

## CLI command system (`apps/cli`)

Commands are objects implementing the `Command` interface
(`command.h`): each `register_command(parent, ctx)` attaches its subcommand,
options, and callback onto a parent `CLI::App`. Because the parent can be the
root app *or* another command's app, commands **nest to any depth**.

- **`Command`** — a leaf unit of functionality (e.g. `greet`, `tui`).
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

The `tui` command is the seam: its callback calls `hestia::tui::run()` and stores
the result as the exit code. That single call is the *only* CLI→TUI reference in
the codebase.

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

- **Everything builds from one configure** — no `BUILD_*` toggles. The
  consequence: every configure fetches CEF (~1 GB, first run only) and requires a
  built `frontend/dist`, so **build the frontend before configuring** (CMakeRC
  globs `dist/` at configure time; a missing `dist/` is a hard `FATAL_ERROR`):
  `(cd apps/desktop/frontend && bun run build)` → `cmake … -B build` → `cmake --build`.
- Each library/app sets `-Wall -Wextra -Wpedantic` (GNU/Clang) or `/W4` (MSVC).
  CEF discovery is isolated inside `apps/desktop/` so its compiler/linker flags
  never reach the shared library, the engine, the CLI, or the TUI.
- The root `CMakeLists.txt` defines the `APP_*` identity (name, id, vendor,
  channel; version from `project(... VERSION)`). `hestia_shared` generates
  `<hestia/app_info.h>` from these and exposes it on its **public** interface, so
  every frontend shares one source of truth — the CLI's `--version`, the desktop's
  `app.info`, etc. all read the same `APP_VERSION`.
- Dependencies that don't appear in a target's public headers are linked
  `PRIVATE` (e.g. shared's spdlog/fmt, tui's ftxui/spdlog). They still propagate as
  transitive link deps to the final executable — which is why `apps/cli` no
  longer links ftxui directly.
- Build artifacts land in `build/<config>/` (e.g. `build/Release/`); the desktop
  binary and its CEF runtime files are copied alongside it.

## See also

- [contributing.md](contributing.md): step-by-step recipes for adding views,
  layouts, components, overlays, and CLI commands.
