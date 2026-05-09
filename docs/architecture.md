# Hestia Architecture

## Overview

Hestia is a Minecraft server manager. Docker model: thin CLI client, persistent background daemon that owns all server
processes.

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
├── server
│   ├── create <name> [--version] [--jar] [--port] [--memory]
│   ├── start  <name>
│   ├── stop   <name>
│   ├── restart <name>
│   ├── rm     <name>
│   ├── ls
│   ├── logs   <name> [-f]
│   ├── console <name>
│   └── inspect <name>
├── daemon
│   ├── start
│   ├── stop
│   └── status
└── config
    ├── get <key>
    └── set <key> <value>
```

Future (plugin system):

```
hestia plugin
    ├── install <name>
    ├── remove  <name>
    └── ls
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

The CLI is a thin HTTP client. All state lives in the daemon.

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
```

All requests/responses: `Content-Type: application/json`.

---

## Storage Layout

```
~/.hestia/
├── config.toml           global config
├── daemon.sock           unix socket       (runtime, deleted on stop)
├── daemon.pid            daemon PID file   (runtime)
└── servers/
    └── <name>/
        ├── hestia.toml   per-server config
        ├── server.jar    minecraft jar
        ├── logs/
        │   └── latest.log
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
jar = "paper"       # paper | vanilla | fabric | forge
memory = "2G"
port = 25565

[jvm]
flags = ["-XX:+UseG1GC"]
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
│   │   ├── root.go               root cobra command + Execute()
│   │   └── commands/
│   │       ├── server/           server subcommands
│   │       ├── daemon/           daemon subcommands
│   │       └── config/           config subcommands
│   ├── daemon/
│   │   ├── daemon.go             daemon lifecycle (start, stop, signal handling)
│   │   ├── api/                  HTTP handlers (one file per resource)
│   │   └── process/              minecraft process management (start, stop, logs)
│   ├── client/
│   │   └── client.go             typed HTTP client over unix socket
│   ├── server/
│   │   ├── config.go             server config struct + TOML marshal/unmarshal
│   │   └── state.go              server runtime state (running, stopped, etc.)
│   └── config/
│       └── config.go             global config (TOML, ~/.hestia/config.toml)
└── pkg/                          public API surface (empty until needed)
```

**Dependency rule**: `cli/commands/*` → `client` → socket → `daemon` → `process`. No layer skips another. CLI never
touches `daemon` or `process` directly.

---

## Plugin System (Future)

Docker-style subprocess plugins: executables named `hestia-<name>` found on `$PATH` or in `~/.hestia/plugins/`.

```
hestia backup ...   →   exec hestia-backup ...
```

`hestia plugin install <name>` fetches and places the binary.

---

## Project Config (Future)

`.hestia.toml` in a project directory pins server settings. `hestia init` creates it. When present, CLI commands use it
as the default server definition.
