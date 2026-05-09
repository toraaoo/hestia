# Hestia

> Docker for Minecraft servers.

Hestia lets you run and manage Minecraft servers from the command line. A persistent background daemon owns your server
processes — they keep running after you close the terminal.

## Features

- Start, stop, and inspect servers with simple commands
- Servers survive terminal exit via background daemon
- Per-server configuration (version, jar type, memory, port)
- Planned: plugin system, project-local config

## Installation

Download the latest release for your platform from the [releases page](https://github.com/toraaoo/hestia/releases).
Extract and place both `hestia` and `hestiad` somewhere on your `$PATH`.

## Usage

```sh
# Start the daemon (once, on login)
hestiad

# Manage servers
hestia server create my-server --version 1.21.4 --jar paper --memory 2G
hestia server start  my-server
hestia server ls
hestia server stop   my-server
hestia server logs   my-server -f

# Daemon control
hestia daemon status
hestia daemon stop

# Configuration
hestia config get daemon.log_level
hestia config set daemon.log_level debug
```

## How it works

```
hestia CLI  ──unix socket──▶  hestiad daemon  ──▶  minecraft process(es)
```

The CLI is a thin client. All state lives in the daemon. Servers are stored under `~/.hestia/servers/`.

## Building from source

Requires Go 1.26+.

```sh
git clone https://github.com/toraaoo/hestia
cd hestia
make build        # produces dist/hestia and dist/hestiad
make install      # installs to $GOBIN
```

## Contributing

```sh
make test   # run tests
make lint   # run linter
```

See [docs/architecture.md](docs/architecture.md) for codebase structure.
