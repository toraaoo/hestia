# Hestia

A Minecraft launcher built in Rust.

Alongside a desktop UI (Tauri), Hestia ships a first-class **CLI** front-end, so
it's just as comfortable from a terminal as from a window.

> **Status:** early development (`v0.0.1`), now a fully all-Rust workspace — the
> C++ tree is gone. Hestia runs as a daemon (`hestiad`) with thin clients over a
> local socket. In place today: the build/workspace, logging, a config store, the
> CLI, Java runtime management (install/list/uninstall via the Adoptium API),
> Microsoft account sign-in, a process supervisor whose workloads survive
> daemon restarts (on-disk records + re-adoption), and full
> Minecraft **server** and **instance** management — a server is fully
> provisioned at create (jar + java runtime + EULA), runs on its own claimed
> port under the supervisor, and has an RCON-backed console (interactive
> attach, one-shot commands, followed logs); an instance materialises its
> files (client jar, libraries,
> assets) at launch and runs as the signed-in account. Both move between game
> versions in place (`server|instance update`; downgrades warn — worlds and
> saves do not downgrade, and both are backed up automatically first). Both
> also have **backups**: on-demand archive/restore of the entry's game data
> (a running server keeps running — world saving pauses around the archive),
> plus per-server scheduled backups with retention pruning. **Content** —
> mods, resourcepacks, shaders, datapacks — is discovered on Modrinth (search,
> browse, resolve versions) and installed into a server (mods, datapacks) or
> instance (mods/resourcepacks/shaders/datapacks) from a project, a Modrinth
> page URL, or a local file, with required dependencies pulled in and a `data/`
> mirror that survives backup/restore (datapacks install straight into their
> world, which the world backup already covers). Vanilla and Fabric are the
> shipped flavors, Modrinth the shipped content source. Still to come:
> installing a whole modpack, wiring the stock Tauri desktop shell to the
> daemon, and a functional tray.

## Front-ends

Hestia is one daemon-backed core with several ways to drive it:

- **CLI** (`hestia`) — scriptable command-line interface for automation and power
  users.
- **Desktop** (`hestia-desktop`) — a Tauri shell hosting a web UI. Scaffolded; the
  frontend is a stock template not yet wired to the daemon.
- **Tray** — a resident system-tray helper (placeholder; not yet ported).

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
│   ├── engine/                config·cache·download·java·accounts   (daemon-only)
│   ├── cli/                   bin: hestia   (clap)
│   ├── daemon/                bin: hestiad  (router, services, supervisor)
│   ├── tray/                  bin: tray     (placeholder)
│   └── desktop/               bin: hestia-desktop (Tauri v2 shell)
├── frontend/                  desktop UI (React + Vite + TS) — self-contained
└── docs/                      architecture, contributing
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
- **[docs/architecture.md](docs/architecture.md)** — the target graph and the
  daemon/engine boundary.
- **[docs/contributing.md](docs/contributing.md)** — conventions and recipes.

## License

[MIT](LICENSE) © 2026 toraaoo
