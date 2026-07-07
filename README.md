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
> plus per-server scheduled backups with retention pruning. Vanilla and
> Fabric are the shipped flavors. Still to come: wiring the stock Tauri
> desktop shell to the daemon and a functional tray.

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

## Usage

```bash
hestia                           # help
hestia play                      # launch an instance — the happy path:
                                 #   one instance runs directly, several prompt a pick

# Minecraft accounts (Microsoft sign-in; `auth` is an alias)
hestia account login             # device-code flow — enter the shown code in a browser
hestia account login --sisu      # browser-redirect flow: sign in, paste the redirect back
hestia account list              # signed-in accounts ('*' marks the one launches use)
hestia account switch [name]     # pick the account launches use (prompts when omitted)
hestia account logout <name|uuid>

# Java runtimes (Eclipse Temurin via the Adoptium API)
hestia java releases             # release lines the provider ships
hestia java install 21           # resolve, download, verify, extract, register
hestia java list                 # installed runtimes
hestia java uninstall 21

# Minecraft servers (fully provisioned at create; run under the daemon;
# each server claims its own port, so several run side by side)
hestia server create             # interactive: flavor → version → EULA confirm
hestia server create vanilla 1.21.1 --eula -n smp   # scriptable (-l pins a
                                 #   loader, -p pins the game port, --memory 4G
                                 #   sets -Xms/-Xmx; --motd, --max-players,
                                 #   --difficulty, --gamemode, --seed cover the
                                 #   common properties, --prop KEY=VALUE the rest)
hestia server list               # managed servers, their address and state
hestia server config smp list    # memory, jvm-args, and server.properties keys
hestia server config smp set memory 4G          # applies from the next start
hestia server config smp set motd "hi"          # any server.properties key its
                                                #   version knows (validated
                                                #   against the generated file)
hestia server update smp 1.21.4  # move a server to another version (world,
                                 #   ports, config stay, and the data is
                                 #   backed up automatically first; prompts
                                 #   for anything omitted; a downgrade asks
                                 #   for a confirm — --downgrade for scripts;
                                 #   a running server confirms a
                                 #   stop-update-start — --restart)
hestia server backup create smp  # archive the world + config into the
                                 #   server's backups/ (a running server keeps
                                 #   running; its world saving pauses around
                                 #   the archive)
hestia server backup list smp    # stored backups, newest first
hestia server backup restore smp # replace the data with a backup (prompts for
                                 #   the backup and confirms — --force for
                                 #   scripts; the current jar/libraries stay)
hestia server backup remove smp <backup>
hestia server config smp set backup-interval 6h  # archive the running server
                                 #   on a schedule (m/h/d units; empty
                                 #   disables); scheduled archives beyond
                                 #   backup-retention (default 7) are pruned
hestia server start smp          # immediate spawn (already provisioned)
hestia server attach smp         # interactive console: live logs, type to send
                                 #   commands, Esc detaches (alias: console)
hestia server command smp say hi # one-shot console command (alias: cmd)
hestia server logs smp -n 50     # captured output (-f keeps following)
hestia server status smp | stop smp | restart smp | remove smp
hestia server versions [flavor] | flavors           # browse the catalogue

# Minecraft instances (clients; files materialise at first launch)
hestia instance create           # interactive: flavor → version
hestia instance create fabric 1.21.1 -n modded --memory 4G
hestia instance launch modded    # ensures java/client/libraries/assets, then runs
hestia instance update modded 1.21.4  # move to another version (saves stay
                                 #   and are backed up automatically first;
                                 #   files download at the next launch; a
                                 #   downgrade asks for a confirm)
hestia instance backup create modded  # archive saves + options (instance
                                 #   stopped; on demand only — no schedule)
hestia instance backup list modded | restore modded | remove modded <backup>
hestia instance config modded set jvm-args "-XX:+UseG1GC"  # memory / jvm-args
hestia instance logs modded -n 50 # captured output (-f keeps following)
hestia instance list | info modded | stop modded | restart modded | remove modded
hestia instance versions [flavor] | flavors

# Download cache
hestia cache info | list | clear

# Configuration (typed settings, stored as JSON)
hestia config get <key> | set <key> <value> | list
hestia config get home           # resolved data directory
hestia config set home <dir>     # persist the data dir (empty reverts to default)
hestia config get autostart      # true if the daemon starts at login
hestia config set autostart true # register the daemon to start at login

# Daemon lifecycle — servers and instances keep running across daemon
# stops/restarts and are re-adopted by the next daemon
hestia daemon status | start | restart
hestia daemon stop               # asks about running workloads on a terminal
hestia daemon stop --all         # stop supervised processes too
hestia daemon stop --keep        # leave them running (script-safe)

# Global flags (any position)
hestia -v java list              # verbose / debug logging (-vv for trace);
                                 #   diagnostics also land in logs/hestia.log
hestia -q java list              # errors only on the console
hestia --home /path/to/dir config get home
hestia --version
```

The data directory is resolved as: `--home` → `$HESTIA_HOME` → a persisted
pointer (`config set home`) → the platform default (`~/.hestia`, or
`%APPDATA%\Hestia` on Windows). **Debug builds** anchor the default at
`<workspace>/.hestia` so development never populates the real per-user directory.

## Documentation

- **[docs/architecture.md](docs/architecture.md)** — the target graph and the
  daemon/engine boundary.
- **[docs/contributing.md](docs/contributing.md)** — conventions and recipes.

## License

[MIT](LICENSE) © 2026 toraaoo
