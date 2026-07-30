# Hestia

A Minecraft launcher built in Rust.

Alongside a desktop UI (Tauri), Hestia ships a first-class **CLI** front-end, so
it's just as comfortable from a terminal as from a window.

> **Status:** early development (`v0.0.1`). Hestia runs as a daemon (`hestiad`)
> with thin clients over a local socket, and the vertical slice is complete —
> everything below works end to end.
>
> **Servers and instances.** A server is fully provisioned at create (jar, Java
> runtime, EULA), claims its own port, and has an RCON-backed console. An
> instance materialises its files at launch, runs as the signed-in account, and
> can run several concurrent sessions. Both move between game versions in place;
> downgrades warn, and a server's data is backed up first.
>
> **Flavors.** Vanilla, Fabric and NeoForge on both sides; Paper, Folia, Spigot
> and CraftBukkit for servers. NeoForge builds its game jar locally from the
> installer, and Spigot/CraftBukkit are compiled on your machine with SpigotMC's
> BuildTools (needs `git`) — so a first create on those takes a few minutes.
> Each flavor describes itself over the wire, including the content kinds it
> takes.
>
> **Content.** Mods, plugins, resourcepacks, shaders and datapacks from Modrinth
> or CurseForge, a page URL or a local file, with dependencies resolved.
> CurseForge's API needs a key (`config set content.curseforge-key`), and the
> source is not offered until one resolves. Whole modpacks install into a new or
> existing entry, their mods joining the pool as ordinary updatable content — a
> CurseForge pack from its source or page URL rather than from a file, since only
> CurseForge resolves the ids its manifest names. An instance's pool can be
> sliced into named content profiles, and reusable global profiles apply project
> references across instances.
>
> **Data.** Server backups on demand and on a schedule, with retention pruning.
> An instance travels as one file instead: export it whole, or as a `.mrpack`
> other launchers read, and import back a hestia archive, a `.mrpack`, or a
> Prism/MultiMC instance. Instance settings and worlds are shared across
> instances (`sync`) — files merged, folders linked into one store — switchable
> off wholesale.
>
> **Around it.** Microsoft sign-in with skin and cape management, a process
> supervisor whose workloads survive daemon restarts, signed announcements,
> self-update, a system tray, the full CLI, and the desktop shell with its
> library, entry, browse, skins, news and settings pages.
>
> **Not built yet:** natives extraction for pre-1.19 clients, and the legacy
> asset layout.

## Front-ends

Hestia is one daemon-backed core with several ways to drive it:

- **CLI** (`hestia`) — scriptable command-line interface for automation and power
  users.
- **Desktop** (`hestia-desktop`) — a Tauri shell hosting the React UI, wired to
  the daemon through a generic IPC bridge with a typed TS API and query layer.
- **Tray** (`tray`) — a resident system-tray helper spawned alongside the
  daemon: status, quick actions (start/restart, autostart, quit).

## Project layout

A single cargo workspace. The one-way dependency arrows are enforced by cargo:
a crate that does not list `engine` as a dependency **cannot** reach launcher
logic — only over the socket via `client`.

```
hestia/
├── Cargo.toml                 [workspace] members = ["crates/*"]
├── rust-toolchain.toml        pinned toolchain + clippy/rustfmt
├── deny.toml                  cargo-deny: licenses, bans, advisories
├── crates/
│   ├── proto/                 wire contracts + domain types (serde)
│   ├── ipc/                   transport (unix socket / named pipe) + envelope (tokio)
│   ├── common/                logging (tracing) + app identity + paths
│   ├── client/                typed client SDK (facades over a Session)
│   ├── engine/                config·cache·download·java·accounts·skins·minecraft·content·process (daemon-only)
│   ├── cli/                   bin: hestia   (clap)
│   ├── daemon/                bin: hestiad  (router, services, supervisor)
│   ├── tray/                  bin: tray     (tray-icon + tao)
│   └── desktop/               bin: hestia-desktop (Tauri v2 shell)
├── frontend/                  desktop UI (React + Vite + TS) — self-contained
├── docs/                      architecture (per subsystem) + decisions
└── news/                      published announcements
```

## Tech stack

- **Rust** (edition 2021), **cargo** workspace
- [tokio](https://tokio.rs/) — async runtime (client + daemon)
- [tracing](https://github.com/tokio-rs/tracing) — structured logging
- [clap](https://github.com/clap-rs/clap) — command-line parsing
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP (engine downloader, auth)
- [serde](https://serde.rs/) — the wire/marshalling layer
- [p256](https://github.com/RustCrypto/elliptic-curves) — Xbox proof-key ECDSA
- [Tauri v2](https://tauri.app/) + [React](https://react.dev/) + [Vite](https://vitejs.dev/) — desktop

## Building

```bash
# Clone and build the daemon + CLI (fast — no desktop/webview deps)
git clone <repo-url> && cd hestia
cargo build -p cli -p daemon
```

The `cli`, `daemon`, and `tray` binaries build with plain `cargo` and
cross-compile cleanly. The **desktop** app needs the system webview libraries
(WebKitGTK on Linux, WebView2 on Windows) and the Bun-built frontend; it does
not cross-compile and is built per-OS:

```bash
# Desktop: Tauri drives the frontend build from crates/desktop/tauri.conf.json
cargo install tauri-cli --version '^2'
(cd frontend && bun install)
(cd crates/desktop && cargo tauri build)     # or `cargo tauri dev` for HMR
```

The [`scripts/`](scripts/) helpers wrap all of this: `scripts/build.sh cli`,
`scripts/run.sh daemon serve`, `scripts/run.sh desktop`, `scripts/package.sh`
(Tauri installers + portable archive — see [docs/packaging.md](docs/packaging.md)).
For an interactive loop, `scripts/dev.sh` opens a subshell with `hestia`/`hestiad`
on `PATH` (or `scripts/dev.sh --desktop` for the Tauri shell with frontend HMR).

## Quick start

```bash
hestia                            # help
hestia account login              # sign in (Microsoft device-code flow)
hestia play                       # launch an instance (prompts to pick when several)
```

Create and drive a server or a client instance. Anything a `create` needs but
wasn't given is prompted for on a terminal:

```bash
hestia server create              # interactive: flavor → version → EULA confirm
hestia instance create            # interactive: flavor → version

hestia start <name>               # start a server or launch an instance
hestia stop <name>                # stop whichever it is
hestia logs <name> -f             # follow its captured output
```

The grammar is entry-first — anything that acts on a specific server or instance
names it right after the noun, then the action:

```bash
hestia server smp config set memory 4G   # applies from the next start
hestia server smp backup create          # archive the world + config
hestia instance modded mod add sodium    # install a mod (deps resolved)
```

The **full command reference** — servers, instances, backups, content, Java,
config, and daemon lifecycle — is in **[docs/cli.md](docs/cli.md)**.

The data directory is resolved as: `--home` → `$HESTIA_HOME` → a persisted
pointer (`config set home`) → the platform default (`~/.hestia`, or
`%APPDATA%\Hestia` on Windows). **Debug builds** anchor the default at
`<workspace>/.hestia` so development never populates the real per-user directory.

## Documentation

- **[docs/cli.md](docs/cli.md)** — the complete `hestia` command reference.
- **[docs/architecture.md](docs/architecture.md)** — how Hestia is put together:
  the daemon/engine boundary, the crate graph, and a page per subsystem.
- **[docs/decisions/](docs/decisions/README.md)** — why it is put together that
  way: one entry per architectural choice, with what it replaced.
- **[docs/contributing.md](docs/contributing.md)** — conventions and recipes.
- **[docs/packaging.md](docs/packaging.md)** — installers and release artifacts.
- **[docs/hooks.md](docs/hooks.md)** — the desktop UI's queries layer: hook
  usage for frontend development.
- **[news/README.md](news/README.md)** — how to publish an announcement.

## License

[GPL-3.0-only](LICENSE) © 2026 toraaoo

The desktop skin preview and its thumbnail renderer are ported from
[Modrinth's launcher](https://github.com/modrinth/code) (GPL-3.0-only,
© Rinth, Inc.) — the reason Hestia is GPL rather than MIT.
