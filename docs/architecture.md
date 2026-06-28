# Architecture

This is the reference for Hestia: what exists today, where it lives, and the
reasoning behind the structure.

> **Status:** early development (`v0.0.1`). The frontend skeletons, build
> system, logging, and config store are in place. Launcher functionality is not
> implemented yet.

## One core, three frontends

Hestia is a single domain core driven three ways:

| Frontend    | Target           | Binary    | Built by default |
|-------------|------------------|-----------|------------------|
| Desktop     | `hestia_desktop` | `Hestia`  | yes              |
| CLI         | `hestia_cli`     | `hestia`  | yes (`BUILD_CLI`)|
| TUI         | `hestia_tui`     | *(in `hestia`)* | yes        |

The CLI and TUI ship in **one binary**: `hestia tui` launches the interactive
terminal UI, every other subcommand is plain CLI. The desktop app is a separate
`main()` over the same core.

## Target graph

```
libs/core    → domain logic, zero UI dependencies
libs/tui     → FTXUI app; depends on core ONLY; public surface = hestia::tui::run()
apps/cli     → CLI11 commands + thin main(); depends on core + tui
apps/desktop → separate main() over core
```

The dependency arrow is one-way and **enforced by the build, not by discipline**:

```
core  ◄────  tui
  ▲          ▲
  └──── cli ─┘
```

`libs/tui` physically cannot `#include` a CLI command — it does not link
`apps/cli`, so it cannot see it. `libs/tui` exposes exactly one public header,
`hestia/tui/run.h`; everything under `libs/tui/src/` is private to the library.
This is why `apps/cli` reaches the TUI through a single symbol and never touches
its internals.

## Directory layout

```
hestia-cpp/
├── CMakeLists.txt              top-level: standard, output dirs, subdirectories
├── cmake/                      module path (custom CMake modules go here)
├── third_party/               vendored deps as git submodules (see below)
├── libs/
│   ├── core/                  hestia_core — shared engine
│   │   ├── include/hestia/    PUBLIC headers (logging.h, config.h, greeting.h)
│   │   └── src/               implementations
│   └── tui/                   hestia_tui — terminal UI library
│       ├── include/hestia/tui/run.h   the ONLY public header
│       └── src/               private internals (see "TUI internals" below)
├── apps/
│   ├── cli/                   hestia_cli — CLI11 commands + main()
│   └── desktop/               hestia_desktop — graphical launcher (stub)
└── docs/
    ├── architecture.md        this file
    └── contributing.md        conventions + how-to recipes
```

### Tech stack

- **C++20**, **CMake** (≥ 3.21), built with **Ninja**.
- [spdlog](https://github.com/gabime/spdlog) + [fmt](https://github.com/fmtlib/fmt) — logging and formatting.
- [CLI11](https://github.com/CLIUtils/CLI11) — command-line parsing.
- [FTXUI](https://github.com/ArthurSonzogni/FTXUI) — terminal user interface.

All four are vendored as git submodules under `third_party/` and added with
`add_subdirectory`. Their tests/examples/docs and install rules are turned off in
`third_party/CMakeLists.txt`.

### Namespaces

| Namespace               | Home            | Contents                              |
|-------------------------|-----------------|---------------------------------------|
| `hestia`                | `libs/core`     | cross-cutting (`init_logging`, `LogLevel`) |
| `hestia::config`        | `libs/core`     | data-dir resolution + `Config` store  |
| `hestia::greeting`      | `libs/core`     | the demo `greet()` function           |
| `hestia::cli`           | `apps/cli`      | command framework + commands          |
| `hestia::tui`           | `libs/tui`      | the terminal UI (everything)          |
| `hestia::tui::keys`     | `libs/tui`      | global key predicates                 |
| `hestia::tui::layout`   | `libs/tui`      | layout id constants                   |
| `hestia::tui::overlay`  | `libs/tui`      | overlay id constants                  |

## Core library (`libs/core`)

UI-free domain logic. Three modules today:

- **`logging`** — `init_logging(LogLevel)` configures the process-wide spdlog
  logger once at startup. `LogLevel` is Hestia's own enum so callers don't depend
  on spdlog's; `logging.cc` maps it across.
- **`config`** — data-directory resolution and a flat `key=value` store.
  Resolution precedence (`data_home`): `--home` override → `$HESTIA_HOME` → a
  persisted pointer file under the anchor dir (`~/.hestia` or `%APPDATA%\Hestia`)
  → the platform default. `Config::load` / `get` / `set` / `save` operate on the
  file at `config_path()`. A missing file is an empty config, not an error.
- **`greeting`** — `greet(name)`; a placeholder exercising the core→CLI seam.

Core links spdlog and fmt **privately** — they are implementation details and do
not leak through its public headers.

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

## Build & dependency conventions

- Each library/app sets `-Wall -Wextra -Wpedantic` (GNU/Clang) or `/W4` (MSVC).
- Dependencies that don't appear in a target's public headers are linked
  `PRIVATE` (e.g. core's spdlog/fmt, tui's ftxui/spdlog). They still propagate as
  transitive link deps to the final executable — which is why `apps/cli` no
  longer links ftxui directly.
- Build artifacts land in `build/<config>/` (e.g. `build/Release/`).
- `HESTIA_VERSION` is injected into `hestia_cli` from the CMake project version.

## See also

- [contributing.md](contributing.md): step-by-step recipes for adding views,
  layouts, components, overlays, and CLI commands.
