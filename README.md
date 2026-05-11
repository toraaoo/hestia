# Hestia

[![Go](https://github.com/toraaoo/hestia/actions/workflows/go.yaml/badge.svg)](https://github.com/toraaoo/hestia/actions/workflows/go.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Docker for Minecraft servers.**

Hestia is a CLI tool for managing Minecraft servers. A persistent background daemon owns your server processes — they keep running after you close the terminal.

![Demo](docs/assets/demo.gif)

## Features

- **Simple CLI** — Create, start, stop servers with single commands
- **Persistent daemon** — Servers survive terminal exit
- **Multiple JAR types** — Vanilla, Paper, Fabric
- **Auto JRE management** — Downloads correct Java version automatically
- **Backup system** — Create, restore, prune backups with retention policies
- **RCON support** — Built-in console access and command execution
- **Progress display** — Live download progress for JARs and JREs

## Quick Start

```sh
# Start the daemon
hestiad &

# Create and start a server
hestia server create survival --version 1.21.4 --jar paper --memory 4G

# View running servers
hestia server ps

# Attach to server console
hestia server attach survival

# Stop server
hestia server stop survival
```

![Server Create](docs/assets/server-create.gif)

## Installation

### From Releases

Download the latest release for your platform from the [releases page](https://github.com/toraaoo/hestia/releases).

```sh
# Extract and install
tar -xzf hestia_linux_amd64.tar.gz
sudo mv hestia hestiad /usr/local/bin/
```

### From Source

Requires Go 1.26+.

```sh
git clone https://github.com/toraaoo/hestia
cd hestia
make build      # outputs to dist/
make install    # installs to $GOBIN
```

## Usage

### Server Management

```sh
# Create server (downloads JAR + JRE automatically)
hestia server create <name> [flags]
  --version     Minecraft version (default: latest)
  --jar         JAR type: vanilla, paper, fabric (default: vanilla)
  --memory      Memory allocation (e.g., 2G, 4096M)
  --port        Server port (auto-assigned if 0)
  --detach      Don't attach after creating

# World configuration
  --world       World name
  --seed        World seed
  --gamemode    survival, creative, adventure, spectator
  --difficulty  peaceful, easy, normal, hard
  --max-players Maximum players
  --motd        Server message of the day

# RCON options
  --rcon            Enable RCON
  --no-rcon         Disable RCON
  --rcon-password   RCON password
  --rcon-port       RCON port

# Lifecycle
hestia server start <name>
hestia server stop <name>
hestia server restart <name>
hestia server rm <name>

# Monitoring
hestia server ps                  # List all servers
hestia server inspect <name>      # Show server details
hestia server logs <name> [-f]    # View logs (-f to follow)
hestia server attach <name>       # Attach to console
hestia server console <name>      # RCON console
```

![Server PS](docs/assets/server-ps.gif)

### Backups

```sh
# Create backup
hestia server backup create <name>
  --full    Full backup (world + config + plugins)
  --force   Force backup without RCON (unsafe)

# Manage backups
hestia server backup list <name>
hestia server backup restore <name> <backup>
hestia server backup delete <name> <backup>

# Prune old backups
hestia server backup prune <name>
  --keep-last    Keep N most recent
  --keep-days    Keep backups newer than N days
  --min-backups  Always keep at least N backups
```

### Versions

```sh
# List available versions
hestia versions
  --jar        Filter by JAR type (vanilla, paper, fabric)
  --snapshots  Include snapshots
  --latest     Show only latest versions
  --json       Output as JSON
```

### Daemon

```sh
hestia daemon start    # Start daemon
hestia daemon stop     # Stop daemon
hestia daemon status   # Check if running
```

### Configuration

```sh
hestia config get <key>
hestia config set <key> <value>
```

## Architecture

```
hestia CLI  ──HTTP/Unix socket──▶  hestiad daemon  ──▶  minecraft process(es)
```

The CLI is a thin HTTP client. All state lives in the daemon. Servers stored under `~/.hestia/servers/`.

### Storage Layout

```
~/.hestia/
├── config.toml           # Global config
├── daemon.sock           # Unix socket (runtime)
├── daemon.pid            # PID file (runtime)
├── jre/                  # Downloaded JREs
│   └── java-21/
└── servers/
    └── <name>/
        ├── hestia.toml   # Server config
        ├── server.jar
        ├── backups/
        ├── logs/
        └── world/
```

### Configuration Files

**Global** — `~/.hestia/config.toml`

```toml
[daemon]
sock = "~/.hestia/daemon.sock"
log_level = "info"
```

**Per-server** — `~/.hestia/servers/<name>/hestia.toml`

```toml
name = "survival"
version = "1.21.4"
jar = "paper"
memory = "4G"
port = 25565

[jvm]
flags = ["-XX:+UseG1GC"]

[rcon]
enabled = true
port = 25575
```

## JAR Providers

| Provider | Description |
|----------|-------------|
| `vanilla` | Official Mojang server |
| `paper` | High-performance Paper server |
| `fabric` | Fabric mod loader |

## Documentation

- [Architecture](docs/architecture.md) — Internal design and package structure
- [Command Reference](docs/commands.md) — Full command documentation
- [Contributing](CONTRIBUTING.md) — Development setup and guidelines

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

```sh
make test   # Run tests
make lint   # Run linter
```

## License

[MIT](LICENSE)
