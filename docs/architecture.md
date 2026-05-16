# Hestia Architecture

## Overview

Hestia is a Minecraft server manager. Docker model: thin CLI client, persistent background daemon that owns all server
processes. Runtime construction is explicit: `internal/app` resolves paths, loads config, and wires services for both
the CLI and daemon.

---

## Binaries

| Binary    | Entry          | Role                                                   |
|-----------|----------------|--------------------------------------------------------|
| `hestia`  | `cmd/hestia/`  | CLI — user-facing, stateless, talks to daemon          |
| `hestiad` | `cmd/hestiad/` | Daemon — owns server processes, survives terminal exit |

---

## Command Surface

```
hestia
├── create <name> [--version] [--jar] [--port] [--memory] ...
├── start <name>
├── stop <name>
├── restart <name>
├── rm <name>
├── ps (aliases: ls, list)
├── logs <name> [-f]
├── attach <name> [--rcon]
├── inspect <name>
├── configure <name> [key] [value]
├── upgrade <name> [version]
├── mod
│   ├── install <server> <mod>
│   ├── list <server>
│   └── remove <server> <mod>
├── backup
│   ├── create <name>
│   ├── list <name>
│   ├── restore <name> <backup>
│   ├── delete <name> <backup>
│   └── prune <name>
├── versions
├── version
├── daemon
│   ├── start
│   ├── stop
│   └── status
└── config
    ├── get <key>
    └── set <key> <value>
```

---

## Data Flow

```
hestia CLI
    │
    │  HTTP/1.1 over Unix socket
    ▼
hestiad daemon  (~/.hestia/daemon.sock)
    │
    ├── manages: minecraft process A
    ├── manages: minecraft process B
    └── manages: minecraft process N
```

The CLI is a thin HTTP client. Command construction receives an owned client and jar provider registry from the CLI app;
commands do not load config, build socket clients, or read server files directly. All server runtime state lives behind
the daemon API.

### API Endpoints

```
POST   /servers                   create server
GET    /servers                   list servers + status
GET    /servers/{name}            inspect server
POST   /servers/{name}/start      start server
POST   /servers/{name}/stop       stop server
POST   /servers/{name}/restart    restart server
DELETE /servers/{name}            remove server
GET    /servers/{name}/logs       stream logs (chunked)
POST   /servers/{name}/upgrade    upgrade server version

GET    /servers/{name}/backups         list backups
POST   /servers/{name}/backups         create backup
POST   /servers/{name}/backups/restore restore backup
DELETE /servers/{name}/backups/{name}  delete backup
POST   /servers/{name}/backups/prune   prune old backups

GET    /servers/{name}/mods       list mods
POST   /servers/{name}/mods       install mod
DELETE /servers/{name}/mods/{mod} remove mod
```

All requests/responses: `Content-Type: application/json`.

---

## Storage Layout

```
~/.hestia/
├── config.toml           global config
├── daemon.sock           unix socket       (runtime, deleted on stop)
├── daemon.pid            daemon PID file   (runtime)
├── jre/                  downloaded JREs
│   └── java-21/
└── servers/
    └── <name>/
        ├── hestia.toml   per-server config
        ├── server.jar    minecraft jar
        ├── backups/
        ├── logs/
        │   └── latest.log
        ├── mods/         mods and plugins
        └── world/
```

---

## Configuration

### Global — `~/.hestia/config.toml`

```toml
[daemon]
sock = "~/.hestia/daemon.sock"
log_level = "info"
```

### Per-server — `~/.hestia/servers/<name>/hestia.toml`

```toml
name = "survival"
version = "1.21.4"
jar = "paper"       # paper | vanilla | fabric
memory = "2G"
port = 25565

[jvm]
flags = ["-XX:+UseG1GC"]

[rcon]
enabled = true
port = 25575
```

---

## Package Structure

```
hestia/
├── cmd/
│   ├── hestia/main.go            CLI entry — calls cli.Execute()
│   └── hestiad/main.go           Daemon entry — calls daemon.Run()
├── internal/
│   ├── cli/
│   │   ├── root.go               CLI app wrapper + root cobra command
│   │   └── commands/
│   │       ├── server/           server lifecycle commands
│   │       ├── daemon/           daemon subcommands
│   │       ├── config/           global config subcommands
│   │       └── versions/         version listing
│   ├── app/
│   │   ├── paths.go              data/config/socket path resolution
│   │   ├── cli.go                CLI dependency graph
│   │   ├── daemon.go             daemon dependency graph
│   │   └── shutdown.go           close-once shutdown signaling
│   ├── daemon/
│   │   ├── daemon.go             HTTP daemon lifecycle only
│   │   ├── api/                  handler structs with injected services
│   │   └── process/              process manager with injected store/JRE/jar services
│   ├── client/
│   │   └── client.go             typed HTTP client over unix socket
│   ├── server/
│   │   ├── config.go             server config values and pure defaults
│   │   ├── properties.go         pure server.properties generation
│   │   └── storage.go            filesystem-backed Store
│   ├── backup/
│   │   ├── backup.go             Service-owned backup creation and restoration
│   │   ├── retention.go          retention policy logic
│   │   └── scheduler.go          backup scheduling with injected store/service
│   ├── jar/
│   │   ├── registry.go           JAR provider registry
│   │   └── providers/            vanilla, paper, fabric providers
│   ├── jre/
│   │   ├── manager.go            JRE root ownership
│   │   └── downloader.go         injectable JRE download logic
│   ├── download/
│   │   └── download.go           owned download client
│   ├── httpc/
│   │   └── client.go             owned HTTP client helper
│   └── config/
│       └── config.go             global config (TOML, ~/.hestia/config.toml)
└── pkg/                          public API surface (empty until needed)
```

**Dependency rule**: `cli/commands/*` → `client` → socket/pipe → `daemon/api` → `daemon/process`. No layer skips another.
CLI never touches server storage, daemon internals, or process state directly.

Stateful behavior is owned by concrete structs:

- `server.Store` owns all server filesystem paths and reads/writes.
- `jre.Manager` owns the JRE root and downloader.
- `backup.Service` owns backup locks, store access, RCON dialing, and clock.
- `process.Manager` receives store, JRE, and jar registry collaborators.
- `api.Handler` receives all daemon services and has no mutable package-level dependencies.
- `daemon.Daemon` owns the listener/server lifecycle and uses `app.Shutdown` for idempotent shutdown.

Package-level functions remain for pure helpers such as config defaults, property generation, version selection, parsing,
and formatting.

---

## Plugin System (Future)

Docker-style subprocess plugins: executables named `hestia-<name>` found on `$PATH` or in `~/.hestia/plugins/`.

```
hestia myplugin ...   →   exec hestia-myplugin ...
```

`hestia plugin install <name>` fetches and places the binary.

---

## Project Config (Future)

`.hestia.toml` in a project directory pins server settings. `hestia init` creates it. When present, CLI commands use it
as the default server definition.
