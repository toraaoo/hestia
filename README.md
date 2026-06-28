# Hestia

A Minecraft launcher built in modern C++.

Alongside a beautiful desktop UI, Hestia ships first-class **CLI** and **TUI**
front-ends, so it's just as comfortable from a terminal as from a window.

> **Status:** early development (`v0.0.1`). The project is being scaffolded —
> the build system, logging, a config store, and the CLI/TUI skeleton are in
> place. Launcher functionality is not implemented yet. Expect things to change.

## Front-ends

Hestia is one core with three ways to drive it:

- **Desktop** (`Hestia`) — the graphical launcher. The primary, "beautiful UI"
  experience.
- **CLI** (`hestia`) — scriptable command-line interface for automation and
  power users.
- **TUI** (`hestia tui`) — a full interactive terminal interface for working
  over SSH or without a desktop session.

The CLI and TUI live in the **same binary**: running `hestia` with no subcommand
shows usage, and `hestia tui` launches the interactive interface.

## Project layout

```
hestia-cpp/
├── libs/core/      hestia_core — shared launcher logic (the engine)
├── libs/tui/       hestia_tui  — terminal UI library (FTXUI)
├── apps/desktop/   Hestia      — graphical desktop launcher
├── apps/cli/       hestia      — CLI + the `tui` subcommand (CLI11)
└── third_party/    vendored dependencies (git submodules)
```

The dependency arrow is one-way and enforced by the build: `tui` and `cli` both
depend on `core`; `cli` links `tui`; nothing depends back on `cli`. The TUI
exposes exactly one public symbol, `hestia::tui::run()`. See
[docs/architecture.md](docs/architecture.md) for the full picture.

## Tech stack

- **C++20**, **CMake** (≥ 3.21), built with Ninja
- [spdlog](https://github.com/gabime/spdlog) + [fmt](https://github.com/fmtlib/fmt) — logging and formatting
- [CLI11](https://github.com/CLIUtils/CLI11) — command-line parsing
- [FTXUI](https://github.com/ArthurSonzogni/FTXUI) — terminal user interface

Dependencies are vendored as git submodules under `third_party/`.

## Building

Clone with submodules:

```bash
git clone --recurse-submodules <repo-url>
cd hestia-cpp
# already cloned? fetch the submodules:
git submodule update --init --recursive
```

Configure and build:

```bash
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build
```

Binaries land in `build/Release/` (or `build/<config>/`):

- `hestia` — CLI / TUI
- `Hestia` — desktop launcher

### Build options

| Option      | Default | Description                   |
|-------------|---------|-------------------------------|
| `BUILD_CLI` | `ON`    | Build the CLI/TUI application |

## Usage

```bash
# Show help
hestia

# Launch the interactive terminal UI
hestia tui

# A friendly greeting (demo command)
hestia greet --name Ada

# Configuration (flat key=value store)
hestia config set <key> <value>
hestia config get <key>
hestia config home              # print the resolved data directory
hestia config set-home <dir>    # persist the data dir for future runs

# Logging verbosity (global flags, accepted at any position)
hestia -v greet   # verbose / debug logging
hestia -q greet   # warnings and errors only

# Override the data directory for one run
hestia --home /path/to/dir config home

# Version
hestia --version
```

The data directory is resolved as: `--home` → `$HESTIA_HOME` → a persisted
pointer (`config set-home`) → the platform default (`~/.hestia`, or
`%APPDATA%\Hestia` on Windows).

## Documentation

- **[docs/architecture.md](docs/architecture.md)** — the as-built map: target
  graph, core/TUI/CLI boundaries, and the TUI's component model.
- **[docs/contributing.md](docs/contributing.md)** — conventions and step-by-step
  recipes for adding a view, layout, component, overlay, or CLI command.

## License

[MIT](LICENSE) © 2026 toraaoo
