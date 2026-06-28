# Contributing — conventions & recipes

Practical, copy-and-adapt guides for extending Hestia. Read
[architecture.md](architecture.md) first for the lay of the land. This document
is the *how*: add a TUI view, layout, component, or overlay; a CLI command; a
core module; or a desktop feature (IPC channel + frontend view).

## Conventions

A few rules hold everywhere; the recipes below assume them.

- **One thing per file.** A view, layout, component, or command is its own
  `.{h,cc}` pair (header-only when there's no implementation, like `keymap.h`).
- **Wire-in is one line.** Each kind of thing has exactly one registry function
  where it's added: `make_views()`, `make_layouts()`, `make_commands()`. Adding a
  feature should not touch the shell.
- **Namespaces follow the target.** Core is `hestia` / `hestia::config` /
  `hestia::greeting`; the CLI is `hestia::cli`; the TUI is `hestia::tui`; the
  desktop shell is `desktop::` (`desktop::ipc`, `desktop::features`, …).
- **Identity comes from one header.** Product name/version/etc. are macros in the
  generated `<hestia/app_info.h>` (`APP_NAME`, `APP_VERSION`, `APP_ID`, …) — use
  them instead of hard-coding or re-injecting per target.
- **Include order:** the matching header first, then a blank line, then standard
  library, then third-party (`<ftxui/...>`, `<spdlog/...>`, `<CLI/...>`), then
  Hestia headers. Within `libs/tui`, include private headers by their path
  relative to `src/` (e.g. `#include "navigation/view.h"`), since `src` is on the
  private include path.
- **Warnings are on** (`-Wall -Wextra -Wpedantic`). Keep them clean.
- **Add new source files to the target's `CMakeLists.txt`.** The lists are
  explicit (no globbing), grouped by subsystem — add your file to the matching
  group.
- **TUI styling goes through the theme.** Never hard-code a color; pull a
  `Decorator` from `theme/theme.h` (`theme.brand`, `theme.muted`, …) so the UI
  keeps honoring the user's terminal palette.
- **TUI state ownership is binary:** shared → `AppContext`; otherwise → a
  component-local member field. Nothing ad-hoc-global.

---

## Add a TUI view

A view is a route-level screen (a "page"). This is the most common addition.

**1. Create `libs/tui/src/views/launcher_view.{h,cc}`.** Implement the `View`
interface. `id()` is the stable route key, `title()` is the sidebar label,
`build(ctx)` constructs the component subtree once.

```cpp
// launcher_view.h
#pragma once
#include "navigation/view.h"

namespace hestia::tui {
    class LauncherView : public View {
    public:
        RouteId id() const override;
        std::string title() const override;
        ftxui::Component build(AppContext &ctx) override;
        // LayoutId layout() const override;  // optional — see "Pick a layout"
    };
}
```

```cpp
// launcher_view.cc
#include "views/launcher_view.h"

#include <ftxui/component/component.hpp>
#include <ftxui/dom/elements.hpp>

#include "app_context.h"
#include "components/panel.h"
#include "theme/theme.h"

namespace hestia::tui {
    RouteId LauncherView::id() const { return "launcher"; }
    std::string LauncherView::title() const { return "Launcher"; }

    ftxui::Component LauncherView::build(AppContext &ctx) {
        using namespace ftxui;

        // Interactive widgets go in a Container so they receive focus/events...
        auto container = Container::Vertical({/* buttons, inputs, ... */});

        // ...then a Renderer rebuilds the Element tree every frame from state.
        return Renderer(container, [&ctx] {
            const Theme &theme = *ctx.theme;
            auto body = vbox({
                text("Instances") | theme.normal,
                filler(),
            });
            return panel("Launcher", body, theme) | flex;
        });
    }
}
```

Key points:

- **Build once, render every frame.** Structure/focus/handlers live in the
  component built by `build()`; the per-frame lambda only reads state and returns
  `Element`s. Don't recreate components inside the lambda.
- **Local state is a member field** on the view class; mutate it from event
  handlers, and the next frame reflects it. Shared state comes from `ctx`.
- **A view with no widgets** still needs a host component — use an empty
  `Container::Vertical({})` (see `about_view.cc`).
- **Wrap content in `panel(...)`** for the standard rounded frame, or arrange
  freely.

**2. Register it in `make_views()`** (`navigation/view_registry.cc`) — order is
sidebar order:

```cpp
views.push_back(std::make_unique<LauncherView>());
```

**3. Add both files** to `libs/tui/CMakeLists.txt` under the `# views` group.

That's it — no shell, navigator, or layout changes. The sidebar, routing, and
lifecycle hooks pick it up automatically.

### Pick a layout

By default a view uses the `Sidebar` shell. To swap, override one method:

```cpp
LayoutId layout() const override { return layout::Centered; }
```

Built-in ids (`layout/layout.h`): `Sidebar` (default chrome), `Fullscreen`
(content only), `Centered` (boxed & centered — good for wizards/dialogs). See
`about_view.cc` for a `Centered` example.

### Lifecycle hooks

Override `on_enter()` / `on_exit()` for mount/unmount work (start a refresh, drop
a cache). The `Navigator` fires them on navigation, including the very first view
at startup.

---

## Add a TUI layout

A layout is a **pure arranger**: given the standard `LayoutSlots`, it places
`Element`s. It never touches components, so it can't break focus/event routing.

**1. Create `libs/tui/src/layout/layouts/split_layout.{h,cc}`.**

```cpp
// split_layout.h
#pragma once
#include "layout/layout.h"

namespace hestia::tui {
    class SplitLayout : public Layout {
    public:
        ftxui::Element arrange(const LayoutSlots &slots) const override;
    };
}
```

```cpp
// split_layout.cc
#include "layout/layouts/split_layout.h"

namespace hestia::tui {
    ftxui::Element SplitLayout::arrange(const LayoutSlots &slots) const {
        using namespace ftxui;
        auto base = vbox({
            slots.header,
            slots.content | flex,
            slots.status,
        });
        return apply_overlay(base, slots.overlay);   // always honor overlays
    }
}
```

Always end with `apply_overlay(base, slots.overlay)` so modal placement stays
consistent across layouts. Use only the slots you want — a layout that omits
`slots.sidebar` simply doesn't render the nav rail.

**2. Give it an id** in `layout/layout.h`:

```cpp
namespace layout {
    inline const LayoutId Split = "split";
}
```

**3. Register it in `make_layouts()`** (`layout/layout_registry.cc`):

```cpp
r.add(layout::Split, std::make_unique<SplitLayout>());
```

**4. Add both files** to `libs/tui/CMakeLists.txt` under `# layout system`.

**Use it** from any view by returning `layout::Split` from `layout()`. An unknown
id falls back to `Sidebar` with a logged warning — never a crash.

---

## Add a reusable component

Components in `components/` are **dumb and presentational**: props in via factory
arguments, an `Element` (or a `Component` for interactive ones) out, no
navigation or app logic. Compare `panel` (returns an `Element`) with `pill_button`
(returns a focusable `Component`).

**1. Create `libs/tui/src/components/badge.{h,cc}`** as a free factory function:

```cpp
// badge.h
#pragma once
#include <string>
#include <ftxui/dom/elements.hpp>

namespace hestia::tui {
    struct Theme;
    ftxui::Element badge(const std::string &text, const Theme &theme);
}
```

```cpp
// badge.cc
#include "components/badge.h"
#include "theme/theme.h"

namespace hestia::tui {
    ftxui::Element badge(const std::string &label, const Theme &theme) {
        using namespace ftxui;
        return text(" " + label + " ") | theme.selected | borderRounded;
    }
}
```

Take the `Theme` by const-ref and pull styling from its roles — never a literal
color. **2. Add the files** to the `# reusable presentational components` group
in `libs/tui/CMakeLists.txt`. No registry — just `#include` it from any view.

---

## Add a TUI overlay

An overlay is a transient modal layer stacked above the active view. While one is
open, `with_global_keys` steps aside and the overlay owns input. Model it on
`overlays/confirm_quit.{h,cc}`.

**1. Create `libs/tui/src/overlays/error_dialog.{h,cc}`.** Give it an id and a
factory that builds a self-contained component (buttons in a `Container`, a
`Renderer` for the box, and a `CatchEvent` that handles `Esc`):

```cpp
namespace overlay {
    inline const OverlayId Error = "error";
}
ftxui::Component make_error_dialog(AppContext &ctx);
```

Closing is `ctx.nav->close_overlay()`; a destructive confirm calls
`ctx.exit_app()` or whatever action it gates.

**2. Wire it into the shell in `tui_app.cc`.** Overlays are currently built and
focus-routed by hand (the shell builds one overlay component and a two-entry
focus `Tab`). To add a second overlay you build it alongside `make_confirm_quit`,
include its `Render()` in `slots.overlay` when it's the active overlay, and route
focus to it. Open it from anywhere via `ctx.nav->open_overlay(overlay::Error)`.

> Overlays are the least "registry-driven" subsystem today — there's one demo
> (`confirm_quit`) and the shell wires it explicitly. If you add several, factor
> an overlay registry mirroring `view_registry` / `layout_registry` first.

---

## Add a CLI command

Commands implement `Command` (`apps/cli/src/command.h`):
`register_command(parent, ctx)` attaches a subcommand, its options, and a
callback onto a parent `CLI::App`. Set results through `ctx.exit_code`.

### A leaf command

**1. Create `apps/cli/src/commands/version_command.{h,cc}`:**

```cpp
// version_command.h
#pragma once
#include "command.h"

namespace hestia::cli {
    class VersionCommand : public Command {
    public:
        void register_command(CLI::App &parent, AppContext &ctx) override;
    };
}
```

```cpp
// version_command.cc
#include "commands/version_command.h"
#include <iostream>

namespace hestia::cli {
    void VersionCommand::register_command(CLI::App &parent, AppContext &ctx) {
        auto *cmd = parent.add_subcommand("version", "Print the version");
        // Bind options/flags onto cmd here, e.g.:
        //   cmd->add_option("-n,--name", name_, "Name to greet");
        cmd->callback([&ctx] {
            std::cout << "hestia\n";
            ctx.exit_code = 0;
        });
    }
}
```

Option values that the callback reads are **member fields** on the command (see
`GreetCommand::name_`), bound with `cmd->add_option(...)` and captured via
`[this, &ctx]`. Read global flags and the data dir from `ctx.global` (e.g.
`config::config_path(ctx.global.home)`).

**2. Register it in `make_commands()`** (`apps/cli/src/registry.cc`):

```cpp
commands.push_back(std::make_unique<VersionCommand>());
```

**3. Add both files** to `apps/cli/CMakeLists.txt`.

### A command group (nested subcommands)

For `hestia foo bar`-style nesting, subclass `CommandGroup` and `add()` children
in the constructor — exactly like `ConfigCommand` (`commands/config_command.cc`),
which groups `get`, `set`, `home`, and `set-home`. Children are themselves
`Command`s, so groups nest to any depth. A bare group requires a subcommand and
otherwise prints its own help. Leaf children may be `private` classes inside the
`.cc` (as the config leaves are) when nothing else needs them.

---

## Add a core module

Domain logic lives in `libs/core`, UI-free.

1. Public header in `libs/core/include/hestia/<name>.h` (namespace `hestia` or a
   sub-namespace like `hestia::config`).
2. Implementation in `libs/core/src/<name>.cc`.
3. Add both to `libs/core/CMakeLists.txt`.
4. Keep dependencies (spdlog/fmt) out of the public header — link them `PRIVATE`.

Frontends then consume it: a CLI command includes the header and links
`hestia_core` (already wired); a view does the same. The `greet` /
`GreetCommand` pair is the minimal end-to-end example.

---

## Add a desktop feature

A desktop feature is a **C++ feature module** (one IPC channel group) plus the
**frontend code** that calls it. The shell never changes. Model it on
`AppFeature` (`apps/desktop/src/features/app/`).

### 1. The C++ side — a feature module

Create `apps/desktop/src/features/instances/instances_feature.{h,cc}`. Implement
`Feature`: `Name()` is the channel prefix, `RegisterActions` registers handlers
(pre-scoped to the prefix, so `On("list", …)` → channel `instances.list`).

```cpp
// instances_feature.h
#pragma once
#include "features/feature.h"

namespace desktop::features {
    class InstancesFeature : public Feature {
    public:
        const char *Name() const override { return "instances"; }
        void RegisterActions(ipc::Actions &on) override;
    };
}
```

```cpp
// instances_feature.cc
#include "features/instances/instances_feature.h"

#include "core/ipc/ipc_router.h"
#include <hestia/...>            // call into the engine here

namespace desktop::features {
    void InstancesFeature::RegisterActions(ipc::Actions &on) {
        on("list", [](const ipc::Request &, ipc::Response res) {
            auto d = CefDictionaryValue::Create();
            // ... populate from hestia_core ...
            res.Success(ipc::Dict(d));
        });
    }
}
```

Handlers may answer synchronously (as above) or capture the copyable `Response`
and answer later from another thread. Push events to the page with
`ipc::Emit(browser, "instances.progress", ipc::Int(42))`.

**Register it** in `BuildFeatures()` (`features/feature_registry.cc`):

```cpp
f.push_back(std::make_unique<InstancesFeature>());
```

**Add both files** to `DESKTOP_SRCS_COMMON` in `apps/desktop/CMakeLists.txt`.

### 2. The frontend side — a typed call + view

Add a typed wrapper in `frontend/src/lib/api.ts` (one function per channel) and,
if it's a new screen, a route under `frontend/src/routes/`. Request/response uses
the `invoke()` helper; native→JS events use `on()` (both in
`frontend/src/lib/ipc.ts`):

```ts
// api.ts
export const listInstances = () =>
  invoke<Instance[]>("instances.list", null, { fallback: [] })
```

Pass a `fallback` so the UI still renders outside the CEF shell (plain browser /
`vite preview`). Then call it from a component — via the TanStack Query hooks in
`hooks/use-ipc.ts` for caching, or directly.

### 3. See it

Run a Debug build against the dev server for instant frontend reload while you
iterate (see [Build & run](#build--run)); rebuild the C++ side when you touch a
feature module.

---

## Build & run

One configure builds everything (CLI/TUI **and** desktop). The frontend `dist/`
must exist *before* you configure — CMakeRC embeds it at configure time:

```bash
(cd apps/desktop/frontend && bun install && bun run build)   # first time / after FE changes
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Debug
cmake --build build

build/Debug/hestia tui          # exercise the TUI
build/Debug/hestia greet -n you # exercise a CLI command
build/Debug/Hestia              # exercise the desktop launcher (embedded frontend)
```

Iterate with `cmake --build build` (Ninja rebuilds only what changed). There is
no test target yet; verify by running the binary.

### Desktop hot reload (frontend)

For instant frontend reload, run a **Debug** build against the Vite dev server.
The dev path is Debug-only (compiled out of Release), and the bridge
(`window.cefQuery`) is injected on every origin, so IPC works identically against
the dev server and the embedded build:

```bash
# terminal 1 — Vite with HMR on :5173
(cd apps/desktop/frontend && bun run dev)

# terminal 2 — launch the shell pointed at it
build/Debug/Hestia --dev-url=http://localhost:5173
```

Edits under `frontend/src/` hot-reload with no rebuild. The **C++ shell does not
hot-reload** — rebuild and relaunch (`cmake --build build && build/Debug/Hestia
--dev-url=…`) after backend changes; the Vite server can keep running across
rebuilds. Alternatives to the flag: `APP_DEV_SERVER_URL=http://localhost:5173`
(env), or bake a Debug default with `-DAPP_DEV_SERVER_URL=http://localhost:5173`
at configure time.

## Recording a decision

When a non-trivial architectural choice is made, capture *what* changed and *why*
in [architecture.md](architecture.md) so the reference stays the single source of
truth. Keep the reasoning next to the structure it explains rather than letting
it drift into commit messages or chat logs.
