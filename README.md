# Hestia

[![CI](https://github.com/toraaoo/hestia/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/toraaoo/hestia/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Docker for Minecraft servers.**

Hestia is a CLI tool for managing Minecraft servers. A persistent background daemon owns your server processes — they
keep running after you close the terminal.

<img src="docs/assets/demo.gif" alt="Hestia Demo" width="1280">

## Features

- **Simple CLI** — Create, start, stop servers with single commands
- **Persistent daemon** — Servers survive terminal exit
- **Multiple JAR types** — Vanilla, Paper, Fabric
- **Auto JRE management** — Downloads correct Java version automatically
- **Mod management** — Install and manage mods/plugins
- **Backup system** — Create, restore, prune backups with retention policies
- **RCON support** — Built-in console access and command execution
- **Progress display** — Live download progress for JARs and JREs

## Quick Start

```sh
# Start the daemon
hestiad &

# Create and start a server
hestia create survival --version 1.21.4 --memory 4G

# View running servers
hestia ps

# Attach to server console
hestia attach survival

# Stop server
hestia stop survival
```

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

### Creating Servers

<p align="center">
  <img src="docs/assets/server-create.gif" alt="Server Create" width="800">
</p>

```sh
hestia create <name> [flags]
```

| Flag            | Description                                         |
|-----------------|-----------------------------------------------------|
| `--version`     | Minecraft version (default: latest)                 |
| `--jar`         | JAR type: vanilla, paper, fabric (default: vanilla) |
| `--memory`      | Memory allocation (e.g., 2G, 4096M)                 |
| `--port`        | Server port (auto-assigned if 0)                    |
| `--detach, -d`  | Don't attach after creating                         |
| `--gamemode`    | survival, creative, adventure, spectator            |
| `--difficulty`  | peaceful, easy, normal, hard                        |
| `--seed`        | World seed                                          |
| `--max-players` | Maximum players                                     |
| `--motd`        | Server message of the day                           |

**Examples:**

```sh
# Vanilla server with defaults
hestia create myserver

# Paper server with 4GB RAM
hestia create survival --jar paper --memory 4G --version 1.21.4

# Creative server with seed
hestia create creative --gamemode creative --seed "minecraft" --memory 2G
```

### Server Lifecycle

```sh
hestia start <name>      # Start server
hestia stop <name>       # Stop server
hestia restart <name>    # Restart server
hestia rm <name>         # Remove server
```

### Monitoring Servers

<p align="center">
  <img src="docs/assets/server-ps.gif" alt="Server PS" width="800">
</p>

```sh
hestia ps                     # List all servers (aliases: ls, list)
hestia inspect <name>         # Show server details
hestia logs <name> [-f]       # View logs (-f to follow)
hestia attach <name>          # Attach to console
hestia attach <name> --rcon   # Attach with RCON responses
```

### Server Configuration

```sh
hestia configure <name>              # View config
hestia configure <name> memory 4G    # Set config value
```

### Upgrading Servers

```sh
hestia upgrade <name> [version]
  --version    Target version (default: latest)
  --restart    Restart server after upgrade
  --no-backup  Skip backup of current server.jar
  --force      Skip downgrade confirmation
```

### Mod Management

<p align="center">
  <img src="docs/assets/mods.gif" alt="Mod Management" width="800">
</p>

```sh
hestia mod install <server> <mod>   # Install a mod
hestia mod list <server>            # List installed mods
hestia mod remove <server> <mod>    # Remove a mod
```

### Backups

<p align="center">
  <img src="docs/assets/demo.gif" alt="Backup Management" width="800">
</p>

```sh
# Create backup
hestia backup create <name>
hestia backup create <name> --full    # Full backup (world + config + plugins)

# Manage backups
hestia backup list <name>
hestia backup restore <name> <backup>
hestia backup delete <name> <backup>

# Prune old backups
hestia backup prune <name> --keep-last 5
hestia backup prune <name> --keep-days 7 --min-backups 3
```

### Versions

<p align="center">
  <img src="docs/assets/versions.gif" alt="Version Listing" width="800">
</p>

```sh
hestia versions                      # List vanilla releases
hestia versions --jar paper          # List Paper versions
hestia versions --jar fabric         # List Fabric versions
hestia versions --latest             # Show only latest
hestia versions --snapshots          # Include snapshots
```

### Daemon

```sh
hestia daemon start    # Start daemon
hestia daemon stop     # Stop daemon
hestia daemon status   # Check if running
```

### Global Configuration

```sh
hestia config get <key>
hestia config set <key> <value>
```

## Architecture

```
hestia CLI  ──HTTP/Unix socket──▶  hestiad daemon  ──▶  minecraft process(es)
```

The CLI is a thin HTTP client. All state lives in the daemon.

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
        ├── mods/
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
jar = "vanilla"
memory = "4G"
port = 25565

[jvm]
flags = ["-XX:+UseG1GC"]

[rcon]
enabled = true
port = 25575
```

## JAR Providers

| Provider  | Description                   |
|-----------|-------------------------------|
| `vanilla` | Official Mojang server        |
| `paper`   | High-performance Paper server |
| `fabric`  | Fabric mod loader             |

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
