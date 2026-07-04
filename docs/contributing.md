# Contributing — conventions & recipes

Practical, copy-and-adapt guides for extending Hestia. Read
[architecture.md](architecture.md) first for the lay of the land. This document
is the *how*: add a CLI command; a core module; or a desktop feature (IPC channel
+ frontend view).

## Conventions

A few rules hold everywhere; the recipes below assume them.

- **One thing per file.** A command is its own `.{h,cc}` pair (header-only when
  there's no implementation).
- **Wire-in is one line.** Each kind of thing has exactly one registry function
  where it's added: `make_commands()` (CLI), `make_services()` (daemon),
  `BuildFeatures()` (desktop). Adding a feature should not touch the shell.
- **Namespaces follow the target.** Shared is `hestia` / `hestia::ipc` /
  `hestia::client`; the engine is `hestia::engine`; the CLI is `hestia::cli`; the
  desktop shell is `desktop::` (`desktop::ipc`, `desktop::features`, …).
- **Identity comes from one header.** Product name/version/etc. are macros in the
  generated `<hestia/app_info.h>` (`APP_NAME`, `APP_VERSION`, `APP_ID`, …) — use
  them instead of hard-coding or re-injecting per target.
- **Include order:** the matching header first, then a blank line, then standard
  library, then third-party (`<spdlog/...>`, `<CLI/...>`), then Hestia headers.
- **Warnings are on** (`-Wall -Wextra -Wpedantic`). Keep them clean.
- **Add new source files to the target's `CMakeLists.txt`.** The lists are
  explicit (no globbing), grouped by subsystem — add your file to the matching
  group.

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
        //   cmd->add_option("major", major_, "Major version to install");
        cmd->callback([&ctx] {
            std::cout << "hestia\n";
            ctx.exit_code = 0;
        });
    }
}
```

Option values that the callback reads are **member fields** on the command (see
`JavaInstallCommand::major_`), bound with `cmd->add_option(...)` and captured via
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
which groups `get`, `set`, and `list`. Children are themselves
`Command`s, so groups nest to any depth. A bare group requires a subcommand and
otherwise prints its own help. Leaf children may be `private` classes inside the
`.cc` (as the config leaves are) when nothing else needs them.

---

## Add an engine module

Launcher/domain logic lives in `libs/engine`, UI-free and daemon-internal.
(Cross-cutting code that the daemon *and* clients both need — transport, the
client SDK, identity, logging — goes in `libs/shared` instead.) A domain
subsystem hangs off the `Engine` aggregate root (`hestia::engine::Engine`), which
the daemon owns and hands to every request handler.

The worked example below adds an `instances` domain end-to-end. Model the store
on `Config` and the service/client on the `config` channels.

**1. Write the subsystem** as a `hestia::engine::<Thing>` class — **one flat
public header per domain** in `libs/engine/include/hestia/engine/<domain>.h`
(includes stay two levels: `<hestia/engine/instances.h>`), implementation in
`libs/engine/src/<domain>/`, both added to `libs/engine/CMakeLists.txt`. Internal
helpers are private headers inside `src/<domain>/` (`src` is a PRIVATE include
dir — see `checksum.h`), so the domain can grow without widening
the public API. Take a path under the data dir in the constructor, serialize
access for concurrent clients, and keep deps (fmt) out of the public header (link
them `PRIVATE`).

```cpp
// libs/engine/include/hestia/engine/instances.h
#pragma once

#include <filesystem>
#include <mutex>
#include <string>
#include <vector>

namespace hestia::engine {
    struct Instance {
        std::string id;
        std::string name;
    };

    class InstanceStore {
    public:
        explicit InstanceStore(std::filesystem::path dir);

        std::vector<Instance> list() const;
        void add(const Instance &instance);

        void reload(std::filesystem::path dir);

    private:
        mutable std::mutex mu_;
        std::filesystem::path dir_;
        std::vector<Instance> instances_;
    };
}
```

```cpp
// libs/engine/src/instances/instance_store.cc
#include <hestia/engine/instances.h>

namespace hestia::engine {
    InstanceStore::InstanceStore(std::filesystem::path dir) : dir_(std::move(dir)) {
        // ... load instances_ from dir_ ...
    }

    std::vector<Instance> InstanceStore::list() const {
        std::lock_guard<std::mutex> lk(mu_);
        return instances_;
    }

    void InstanceStore::add(const Instance &instance) {
        std::lock_guard<std::mutex> lk(mu_);
        instances_.push_back(instance);
        // ... persist under dir_ ...
    }

    void InstanceStore::reload(std::filesystem::path dir) {
        std::lock_guard<std::mutex> lk(mu_);
        dir_ = std::move(dir);
        // ... reload instances_ from dir_ ...
    }
}
```

**2. Hang it off `Engine`** (`engine.h` / `engine.cc`): a member constructed in
the initializer list against `data_home_`, a getter, and a `reload()` in
`set_data_home()`. This is the *only* change to the engine's wiring —
`HandlerContext` already carries the `Engine`.

```cpp
// engine.h — inside class Engine
#include <hestia/engine/instances.h>
// ...
        InstanceStore &instances() { return instances_; }
// ...
        Config config_;
        InstanceStore instances_;          // declare after config_ — order = init order
```

```cpp
// engine.cc
Engine::Engine(const std::filesystem::path &override_home)
    : data_home_(paths::data_home(override_home)),
      config_(paths::config_path(data_home_)),
      instances_(data_home_ / "instances") {}

std::filesystem::path Engine::set_data_home(const std::string &dir) {
    paths::set_persisted_home(dir);
    data_home_ = paths::data_home();
    config_.reload(paths::config_path(data_home_));
    instances_.reload(data_home_ / "instances");
    return data_home_;
}
```

**3. Define the wire contract** in
`libs/shared/include/hestia/proto/instances.h` — the single definition both
sides marshal through. A call contract names its channel once and pairs it with
the `Params`/`Result` payload shapes; each payload struct declares its wire
format as a `kFields` table (see `contract.h` for the field flags: `kRequired`,
`kOmitIfEmpty`, `kFlatten`). Add the header (and a `src/proto/instances.cc`
only if the domain has enum/string helpers) to the shared `CMakeLists.txt`:

```cpp
// libs/shared/include/hestia/proto/instances.h
#pragma once

#include <string>
#include <vector>

#include <hestia/proto/contract.h>

namespace hestia::proto {
    struct Instance {
        std::string id;
        std::string name;

        static constexpr auto kFields = fields(field("id", &Instance::id), field("name", &Instance::name));
    };

    struct InstancesList {
        static constexpr const char *kChannel = "instances.list";
        using Params = Empty;
        struct Result {
            std::vector<Instance> instances;

            static constexpr auto kFields = fields(field("instances", &Result::instances));
        };
    };
}
```

**4. Serve it** — a `Service` class, the daemon's flavor of the CLI's `Command`
and the desktop's `Feature`. Add `services/instances_service.{h,cc}`, one line
in `make_services()` (`services/registry.cc`), and both files to the daemon
`CMakeLists.txt`. `Channels::handle<C>` decodes `C::Params` (a malformed
payload answers `bad_request`) and encodes the returned `C::Result`; handlers
reach the daemon's long-lived collaborators through `HandlerContext` and throw
`ServiceError` for a typed failure:

```cpp
// apps/daemon/src/services/instances_service.h
#pragma once

#include "services/service.h"

namespace hestia::daemon {
    class InstancesService : public Service {
    public:
        void register_channels(Channels &on) override;
    };
}
```

```cpp
// apps/daemon/src/services/instances_service.cc
#include "services/instances_service.h"

#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <hestia/engine/engine.h>
#include <hestia/proto/instances.h>

namespace hestia::daemon {
    void InstancesService::register_channels(Channels &on) {
        on.handle<proto::InstancesList>([](const proto::Empty &, HandlerContext &ctx) {
            proto::InstancesList::Result out;
            for (const auto &it: ctx.runtime.engine().instances().list()) {
                out.instances.push_back(proto::Instance{.id = it.id, .name = it.name});
            }
            return out;
        });
    }
}
```

**5. Expose it on the client SDK** — a `Facade` in
`libs/shared/include/hestia/client/instances.h` plus an accessor on `Client`
(`client/client.h`: a member, an accessor, one entry in the constructor's
initializer list; both new files in the shared `CMakeLists.txt`). Facade
methods are one-liners over the typed session call and return `proto` types
directly:

```cpp
// libs/shared/include/hestia/client/instances.h
#pragma once

#include <vector>

#include <hestia/client/facade.h>
#include <hestia/proto/instances.h>

namespace hestia::client {
    class Instances : public Facade {
    public:
        using Facade::Facade;

        std::vector<proto::Instance> list();
    };
}
```

```cpp
// libs/shared/src/client/instances.cc
#include "hestia/client/instances.h"

#include "session.h"

namespace hestia::client {
    std::vector<proto::Instance> Instances::list() {
        return session_->call<proto::InstancesList>().instances;
    }
}
```

Front-ends reach the subsystem only over the socket via the client SDK
(`client.instances().list()`) — they never include the engine header. (The
desktop app still links `hestia_engine` directly today, a transitional
exception; the CLI already goes entirely through the client SDK.) The `config`
channels are the shipped end-to-end reference: `client.config().get(key)`
round-trips `config.get` to the daemon's `ConfigService`, which calls
`ctx.runtime.engine().config().get()`. A stateless helper with no persisted
state can skip step 2 and be called directly from its service.

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
            // ... populate from hestia_engine ...
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

One configure builds everything (CLI **and** desktop). The frontend `dist/`
must exist *before* you configure — CMakeRC embeds it at configure time:

```bash
(cd apps/desktop/frontend && bun install && bun run build)   # first time / after FE changes
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Debug
cmake --build build

build/Debug/hestia java list    # exercise a CLI command
build/Debug/Hestia              # exercise the desktop launcher (embedded frontend)
```

Iterate with `cmake --build build` (Ninja rebuilds only what changed). Unit
tests live in `tests/` (`hestia_tests`, GoogleTest) — run them with
`ctest --test-dir build`; also verify by running the binary.

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
