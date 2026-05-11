# Command Reference

Complete reference for all Hestia commands.

## Global Flags

These flags are available on all commands:

| Flag     | Description           |
|----------|-----------------------|
| `--help` | Show help for command |

## hestia create

Create and start a new server.

```sh
hestia create <name> [flags]
```

**Arguments:**

- `<name>` — Server name (used as directory name and identifier)

**Flags:**

| Flag           | Type   | Default | Description                            |
|----------------|--------|---------|----------------------------------------|
| `--version`    | string | latest  | Minecraft version                      |
| `--jar`        | string | vanilla | JAR provider: vanilla, paper, fabric   |
| `--memory`     | string |         | Memory allocation (e.g., 2G, 4096M)    |
| `--port`       | int    | auto    | Server port                            |
| `--detach, -d` | bool   | false   | Don't attach to console after creating |
| `--json`       | bool   | false   | Output as JSON (no progress)           |

**World Flags:**

| Flag            | Type   | Description                                         |
|-----------------|--------|-----------------------------------------------------|
| `--world`       | string | World directory name                                |
| `--seed`        | string | World generation seed                               |
| `--gamemode`    | string | Game mode: survival, creative, adventure, spectator |
| `--difficulty`  | string | Difficulty: peaceful, easy, normal, hard            |
| `--max-players` | int    | Maximum player count                                |
| `--motd`        | string | Server message of the day                           |

**RCON Flags:**

| Flag              | Type   | Description   |
|-------------------|--------|---------------|
| `--rcon`          | bool   | Enable RCON   |
| `--no-rcon`       | bool   | Disable RCON  |
| `--rcon-password` | string | RCON password |
| `--rcon-port`     | int    | RCON port     |

**Examples:**

```sh
# Create with defaults (latest vanilla)
hestia create myserver

# Paper server with 4GB RAM
hestia create survival --jar paper --memory 4G --version 1.21.4

# Creative server with seed
hestia create creative --gamemode creative --seed "minecraft" --memory 2G

# Create without attaching
hestia create background-server -d
```

## hestia start

Start a stopped server.

```sh
hestia start <name>
```

## hestia stop

Stop a running server.

```sh
hestia stop <name>
```

The server is stopped gracefully via RCON if available, otherwise SIGTERM.

## hestia restart

Restart a server.

```sh
hestia restart <name>
```

## hestia rm

Remove a server and its data.

```sh
hestia rm <name>
```

**Warning:** This deletes the server directory including world data and backups.

## hestia ps

List all servers with status.

```sh
hestia ps [flags]
```

**Aliases:** `ls`, `list`

**Flags:**

| Flag     | Type | Default | Description    |
|----------|------|---------|----------------|
| `--json` | bool | false   | Output as JSON |

**Output columns:**

- NAME — Server name
- STATE — running, stopped, starting, stopping
- VERSION — Minecraft version
- JAR — JAR type
- PORT — Server port
- UPTIME — Time since start (if running)

## hestia inspect

Show detailed server information.

```sh
hestia inspect <name>
```

## hestia logs

View server logs.

```sh
hestia logs <name> [flags]
```

**Flags:**

| Flag           | Type | Default | Description             |
|----------------|------|---------|-------------------------|
| `-f, --follow` | bool | false   | Follow log output       |
| `-n, --lines`  | int  | 100     | Number of lines to show |

## hestia attach

Attach to server console with interactive input.

```sh
hestia attach <name> [flags]
```

**Flags:**

| Flag          | Type | Default | Description                             |
|---------------|------|---------|-----------------------------------------|
| `-n, --lines` | int  | 100     | Number of log lines to show             |
| `--rcon`      | bool | false   | Use RCON for commands (shows responses) |

Press `Ctrl+C` to detach without stopping the server.

## hestia configure

View or modify server configuration.

```sh
hestia configure <name> [key] [value]
```

**Examples:**

```sh
# View all config
hestia configure myserver

# Get specific value
hestia configure myserver memory

# Set value
hestia configure myserver memory 4G
```

## hestia upgrade

Upgrade server to a new version.

```sh
hestia upgrade <name> [version] [flags]
```

**Flags:**

| Flag          | Type   | Default | Description                       |
|---------------|--------|---------|-----------------------------------|
| `--version`   | string | latest  | Target Minecraft version          |
| `--restart`   | bool   | false   | Restart server after upgrade      |
| `--no-backup` | bool   | false   | Skip backup of current server.jar |
| `--force`     | bool   | false   | Skip downgrade confirmation       |
| `--json`      | bool   | false   | Output as JSON                    |

## hestia mod

Manage server mods and plugins.

### mod install

Install a mod to a server.

```sh
hestia mod install <server> <mod>
```

### mod list

List installed mods on a server.

```sh
hestia mod list <server>
```

### mod remove

Remove a mod from a server.

```sh
hestia mod remove <server> <mod>
```

## hestia backup

Manage server backups.

### backup create

Create a backup of the server.

```sh
hestia backup create <name> [flags]
```

**Flags:**

| Flag      | Type | Default | Description                              |
|-----------|------|---------|------------------------------------------|
| `--full`  | bool | false   | Full backup (world + config + plugins)   |
| `--force` | bool | false   | Force backup without RCON flush (unsafe) |
| `--json`  | bool | false   | Output as JSON                           |

By default, creates a world-only backup. Uses RCON to flush world data before backup.

### backup list

List all backups for a server.

```sh
hestia backup list <name> [flags]
```

**Flags:**

| Flag     | Type | Default | Description    |
|----------|------|---------|----------------|
| `--json` | bool | false   | Output as JSON |

### backup restore

Restore a server from backup.

```sh
hestia backup restore <name> <backup>
```

**Arguments:**

- `<name>` — Server name
- `<backup>` — Backup name (from `backup list`)

The server is stopped before restore if running.

### backup delete

Delete a specific backup.

```sh
hestia backup delete <name> <backup>
```

### backup prune

Remove old backups based on retention policy.

```sh
hestia backup prune <name> [flags]
```

**Flags:**

| Flag            | Type | Default | Description                    |
|-----------------|------|---------|--------------------------------|
| `--keep-last`   | int  | 0       | Keep N most recent backups     |
| `--keep-days`   | int  | 0       | Keep backups newer than N days |
| `--min-backups` | int  | 0       | Always keep at least N backups |

**Examples:**

```sh
# Keep last 5 backups
hestia backup prune myserver --keep-last 5

# Keep backups from last 7 days, minimum 3
hestia backup prune myserver --keep-days 7 --min-backups 3
```

## hestia versions

List available Minecraft versions.

```sh
hestia versions [flags]
```

**Flags:**

| Flag          | Type   | Default | Description                       |
|---------------|--------|---------|-----------------------------------|
| `--jar`       | string | vanilla | JAR provider to list versions for |
| `--snapshots` | bool   | false   | Include snapshot versions         |
| `--latest`    | bool   | false   | Show only latest release/snapshot |
| `--json`      | bool   | false   | Output as JSON                    |

**Examples:**

```sh
# List vanilla releases
hestia versions

# List Paper versions
hestia versions --jar paper

# Show only latest
hestia versions --latest

# Include snapshots
hestia versions --snapshots
```

## hestia version

Show hestia version.

```sh
hestia version
```

## hestia daemon

Manage the background daemon.

### daemon start

Start the daemon if not running.

```sh
hestia daemon start
```

### daemon stop

Stop the daemon and all managed servers.

```sh
hestia daemon stop
```

### daemon status

Check if daemon is running.

```sh
hestia daemon status
```

## hestia config

Manage global configuration.

### config get

Get a configuration value.

```sh
hestia config get <key>
```

### config set

Set a configuration value.

```sh
hestia config set <key> <value>
```

**Available keys:**

| Key                | Type   | Description                         |
|--------------------|--------|-------------------------------------|
| `daemon.sock`      | string | Unix socket path                    |
| `daemon.log_level` | string | Log level: debug, info, warn, error |

## Exit Codes

| Code | Meaning       |
|------|---------------|
| 0    | Success       |
| 1    | General error |
