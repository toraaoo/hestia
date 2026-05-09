# Hestia Minecraft Server Management - Implementation Plan

## Context

Hestia has CLI+daemon architecture in place (`hestia` CLI → Unix socket → `hestiad` daemon). Infrastructure exists: daemon lifecycle, HTTP API scaffold, config loading, Cobra commands. Missing: all server management functionality - JRE resolver, process management, server CRUD, logs, RCON.

Goal: Docker-like experience for Minecraft servers without Docker. Each phase is independently executable and testable.

---

## Phase 1: JRE Resolver & Downloader

**Outcome**: `internal/jre/` package that detects required Java version and downloads if needed.

### Files to Create
- `internal/jre/downloader.go` - download JRE from Adoptium API
- `internal/jre/manager.go` - manage installed JREs at `~/.hestia/jre/`, check if version exists, return java path

### Implementation
1. No hardcoded mapping - Java version comes from Mojang version manifest (`javaVersion.majorVersion` field)
2. Use Adoptium API (`https://api.adoptium.net/v3/`) for JRE downloads
3. Store JREs: `~/.hestia/jre/java-{version}/`
4. `GetJRE(majorVersion int) string` - returns path to `java` binary, downloads if missing
5. Phase 2 calls Phase 1: fetch manifest → extract `javaVersion.majorVersion` → pass to JRE manager

### Verification
```bash
# Unit test: version resolution
go test ./internal/jre/...

# Manual: trigger download
# (via temporary CLI command or test)
```

---

## Phase 2: Server Jar Management

**Outcome**: Download vanilla server.jar from Mojang for any version.

### Files to Create
- `internal/jar/manifest.go` - fetch Mojang version manifest + version listing
- `internal/jar/downloader.go` - download server.jar
- `internal/jar/cache.go` - manifest caching with TTL
- `internal/jar/types.go` - interfaces for future Paper/Fabric/Forge support
- `internal/cli/commands/versions.go` - `hestia versions` command
- `internal/daemon/api/versions.go` - `/versions` endpoint

### Implementation
1. Fetch `https://piston-meta.mojang.com/mc/game/version_manifest_v2.json`
2. Parse version list, find version metadata URL
3. Extract `downloads.server.url` from version metadata
4. Download to `~/.hestia/servers/{name}/server.jar`
5. Interface: `JarProvider` for future server types (design now, implement vanilla only)

### Version Listing
```bash
hestia versions                    # list all releases
hestia versions --snapshots        # include snapshots
hestia versions --latest           # show only latest release + snapshot
```

API endpoint:
```
GET /versions?snapshots=false      # list available MC versions
```

Implementation:
1. Cache manifest locally (`~/.hestia/cache/versions.json`) with TTL (1 hour)
2. Filter by release type (release/snapshot)
3. Return: version ID, release date, type, `javaVersion.majorVersion` from manifest

### Verification
```bash
go test ./internal/jar/...
# Manual: download specific version, verify SHA
hestia versions --latest           # should show current release
```

---

## Phase 3: Server Config & Storage

**Outcome**: Per-server configuration and directory structure.

### Files to Create
- `internal/server/config.go` - server config struct + TOML (exists in docs, not code)
- `internal/server/properties.go` - generate server.properties from config
- `internal/server/storage.go` - manage server directories

### Config Structure (hestia.toml)
```toml
name = "survival"
version = "1.21.4"
jar = "vanilla"
memory = "2G"
port = 25565          # auto-resolved if 0

[rcon]
enabled = true
password = "auto-generated-uuid"
port = 25575          # auto-resolved

[world]
name = "world"
seed = ""
gamemode = "survival"
difficulty = "normal"
max_players = 20
motd = "A Minecraft Server"
```

### Implementation
1. Config struct with TOML tags
2. Port resolver: scan 25565-25600, find first free port
3. RCON port resolver: scan 25575-25600
4. Generate `server.properties` from config
5. Auto-generate secure RCON password (UUID v4)
6. Create directory structure per architecture doc

### Verification
```bash
go test ./internal/server/...
# Check: port conflict detection, properties generation
```

---

## Phase 4: Process Management

**Outcome**: Start/stop Minecraft server processes, daemon owns them.

### Files to Create
- `internal/daemon/process/process.go` - process lifecycle (start, stop, signal)
- `internal/daemon/process/state.go` - runtime state tracking
- `internal/daemon/process/io.go` - stdin/stdout/stderr management

### Implementation
1. Process struct: holds `exec.Cmd`, stdin pipe, state
2. Start: build command line
   ```
   {java_path} -Xmx{memory} -Xms{memory} {jvm_flags} -jar server.jar nogui
   ```
3. Stdin pipe for sending commands
4. Stdout/stderr → ring buffer (last N lines) + file log
5. Graceful stop: send `stop` command via stdin, wait 30s, then SIGTERM
6. State machine: `stopped` → `starting` → `running` → `stopping` → `stopped`
7. Process map in daemon: `map[string]*Process`

### Verification
```bash
go test ./internal/daemon/process/...
# Integration: start server manually, verify process runs
```

---

## Phase 5: Server CRUD API

**Outcome**: API endpoints for create/list/inspect/delete servers.

### Files to Modify
- `internal/daemon/api/api.go` - add routes
- `internal/daemon/api/servers.go` (new) - server handlers

### Endpoints
```
POST   /servers          - create server (triggers JRE + jar download)
GET    /servers          - list all servers + status
GET    /servers/{name}   - inspect single server
DELETE /servers/{name}   - remove server (must be stopped)
```

### Implementation
1. Create: validate config → resolve JRE → download jar → write config → respond
2. List: scan `~/.hestia/servers/`, read configs, add runtime state
3. Inspect: full config + state + process info
4. Delete: verify stopped → remove directory

### Verification
```bash
# curl tests against running daemon
curl -X POST localhost/servers -d '{"name":"test","version":"1.21.4"}'
curl localhost/servers
curl localhost/servers/test
curl -X DELETE localhost/servers/test
```

---

## Phase 6: Start/Stop API + CLI

**Outcome**: Start and stop servers via CLI.

### Files to Modify
- `internal/daemon/api/servers.go` - start/stop handlers
- `internal/cli/commands/server/` - CLI commands

### Endpoints
```
POST /servers/{name}/start
POST /servers/{name}/stop
POST /servers/{name}/restart
```

### CLI Commands
```bash
hestia server create <name> --version 1.21.4 [--port] [--memory]
hestia server start <name>
hestia server stop <name>
hestia server restart <name>
hestia server rm <name>
hestia server ls
hestia server inspect <name>
```

### Implementation
1. Start handler: check state → get JRE path → spawn process → update state
2. Stop handler: send stop command → wait → force kill if needed
3. CLI: thin wrappers calling client methods

### Verification
```bash
hestia server create test --version 1.21.4
hestia server start test
hestia server ls  # should show "running"
hestia server stop test
```

---

## Phase 7: Logs & Console

**Outcome**: Tail logs and send commands to running servers.

### Files to Create/Modify
- `internal/daemon/api/logs.go` - log streaming
- `internal/cli/commands/server/logs.go`
- `internal/cli/commands/server/console.go`

### Endpoints
```
GET  /servers/{name}/logs?follow=true&lines=100
POST /servers/{name}/console  {"command": "say hello"}
```

### Implementation
1. Logs: read from ring buffer + file, SSE stream for follow mode
2. Console: write to process stdin pipe
3. CLI `logs -f`: stream SSE events to terminal
4. CLI `console`: interactive mode, readline → POST → display response

### Verification
```bash
hestia server logs test -f        # should stream
hestia server console test        # interactive
# In console: type "list", should see player list
```

---

## Phase 8: RCON Integration

**Outcome**: Send commands via RCON (alternative to stdin console).

### Files to Create
- `internal/rcon/client.go` - RCON client wrapper

### Implementation
1. Use existing Go library: `github.com/gorcon/rcon` or `github.com/willroberts/minecraft-client`
2. Add RCON option to console command: `hestia server console test --rcon`
3. RCON benefits: works even if stdin pipe has issues, standard protocol

### Verification
```bash
hestia server console test --rcon
# Send command, verify response
```

---

## Phase 9: Config Commands

**Outcome**: View and modify server config via CLI.

### Files to Modify
- `internal/cli/commands/config/` - implement get/set
- `internal/daemon/api/config.go` - config endpoints

### Endpoints
```
GET  /servers/{name}/config
PUT  /servers/{name}/config
```

### CLI
```bash
hestia server config test                    # show all
hestia server config test port               # get single
hestia server config test port 25566         # set value
hestia server config test --edit             # open in $EDITOR (future)
```

### Implementation
1. Get: return hestia.toml as JSON
2. Set: update field, regenerate server.properties, require restart if running
3. Port change: re-validate availability

### Verification
```bash
hestia server config test port 25570
hestia server config test port  # should show 25570
```

---

## Critical Files Reference

| File | Purpose |
|------|---------|
| `internal/jre/manager.go` | JRE lifecycle - check/download/path |
| `internal/jre/downloader.go` | Adoptium API JRE download |
| `internal/jar/manifest.go` | Mojang version manifest fetch |
| `internal/server/config.go` | Per-server config struct |
| `internal/server/properties.go` | server.properties generation |
| `internal/daemon/process/process.go` | Process lifecycle |
| `internal/daemon/api/servers.go` | Server CRUD handlers |
| `internal/client/client.go` | Existing - add server methods |
| `internal/cli/commands/versions.go` | Version listing CLI command |

---

## Dependencies to Add

```go
// go.mod additions
github.com/gorcon/rcon        // RCON client (Phase 8)
github.com/google/uuid        // RCON password generation (Phase 3)
```

---

## Testing Strategy

Each phase:
1. Unit tests for new packages
2. Integration test with daemon (where applicable)
3. Manual CLI verification

End-to-end test after Phase 6:
```bash
hestiad &                                    # start daemon
hestia server create test --version 1.21.4  # should download JRE + jar
hestia server start test                     # should start server
sleep 30                                     # wait for server ready
hestia server logs test                      # should show startup logs
hestia server stop test                      # should graceful stop
hestia server rm test                        # should cleanup
```

---

## Sources

- [Minecraft Wiki - RCON](https://minecraft.wiki/w/RCON)
- [Minecraft Wiki - server.properties](https://minecraft.wiki/w/Server.properties)
- [Minecraft Wiki - version_manifest.json](https://minecraft.wiki/w/Version_manifest.json)
- [gorcon/rcon - Go RCON library](https://github.com/gorcon/rcon)
- [MC Server Soft - Java versions](https://docs.mcserversoft.com/advanced/java-version)
- [Go os/exec patterns](https://www.dolthub.com/blog/2022-11-28-go-os-exec-patterns/)
- [Adoptium API](https://api.adoptium.net/)
