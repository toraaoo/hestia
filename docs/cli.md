# CLI reference — `hestia`

The complete command surface of the `hestia` front-end. See the
[README](../README.md) for a quick start and
[architecture.md](architecture.md) for how the CLI sits over the daemon.

## Grammar at a glance

The grammar is **noun-first** and, for anything that touches a specific entry,
**entry-first** — the name sits in one fixed slot right after the noun:

```bash
hestia server create              # catalogue verbs take no entry
hestia server <name> <action>     # everything else names the entry first
hestia server smp start
hestia server smp config set memory 4G
```

Two cross-cutting shortcuts sit on top: `hestia play` (the happy path) and the
verb-first `hestia start|stop|restart|logs|rename <name>`, which resolve a name
across both servers and instances so you need not recall which kind it is.

Conventions: anything a `create` needs but wasn't given is prompted for on a
terminal (piped invocations must pass the flag); `ls`/`rm` alias every
list/remove; `-v`/`-vv` raise log verbosity, `-q` quiets to errors; `--home`
overrides the data directory for an auto-spawned daemon.

## Accounts

Microsoft sign-in; `auth` is an alias for `account`.

```bash
hestia account login             # device-code flow — opens the browser, the code
                                 #   shown and copied to the clipboard
hestia account login --sisu      # browser-redirect flow: sign in, paste the redirect back
hestia account list              # signed-in accounts ('*' marks the one launches use)
hestia account switch [name]     # pick the account launches use (prompts when omitted)
hestia account logout <name|uuid>
```

## Java runtimes

Eclipse Temurin via the Adoptium API.

```bash
hestia java releases             # release lines the provider ships
hestia java install 21           # resolve, download, verify, extract, register
hestia java list                 # installed runtimes
hestia java uninstall 21
```

## Servers

Fully provisioned at create; run under the daemon; each server claims its own
port, so several run side by side. Catalogue verbs take no entry:

```bash
hestia server create             # bare: the fullscreen wizard — flavor →
                                 #   version (type to filter; Tab pulls
                                 #   snapshots in) → name → settings
                                 #   (skippable) → confirm (EULA); Esc steps
                                 #   back; any argument switches to the
                                 #   flag-driven flow below
hestia server create vanilla 1.21.1 --eula -n smp   # scriptable (-l pins a
                                 #   loader, -p pins the game port, --memory 4G
                                 #   sets -Xms/-Xmx; --motd, --max-players,
                                 #   --difficulty, --gamemode, --seed cover the
                                 #   common properties, --prop KEY=VALUE the rest)
hestia server list               # managed servers, their address and state
hestia server versions [flavor]  # game versions a flavor offers
hestia server flavors            # the available flavors
```

Everything that acts on one server is entry-first — `hestia server <name> <action>`:

```bash
hestia server smp config list    # memory, jvm-args, and server.properties keys
hestia server smp config set memory 4G          # applies from the next start
hestia server smp config set motd "hi"          # any server.properties key its
                                                #   version knows (validated
                                                #   against the generated file)
hestia server smp update 1.21.4  # move the server to another version (world,
                                 #   ports, config stay, and the data is
                                 #   backed up automatically first; prompts
                                 #   for anything omitted; a downgrade asks
                                 #   for a confirm — --downgrade for scripts;
                                 #   a running server confirms a
                                 #   stop-update-start — --restart)
hestia server smp backup create  # archive the world + config into the
                                 #   server's backups/ (a running server keeps
                                 #   running; its world saving pauses around
                                 #   the archive)
hestia server smp backup list    # stored backups, newest first
hestia server smp backup restore # replace the data with a backup (prompts for
                                 #   the backup and confirms — --force for
                                 #   scripts; the current jar/libraries stay)
hestia server smp backup remove <backup>
hestia server smp config set backup-interval 6h  # archive the running server
                                 #   on a schedule (m/h/d units; empty
                                 #   disables); scheduled archives beyond
                                 #   backup-retention (default 7) are pruned
hestia server smp start          # immediate spawn, then attaches the console
                                 #   (-d/--detach returns immediately)
hestia server smp attach         # interactive console: live logs, type to send
                                 #   commands, Esc detaches (alias: console)
hestia server smp command say hi # one-shot console command (alias: cmd)
hestia server smp logs -n 50     # captured output (-f opens the fullscreen
                                 #   log session; piped it streams plainly)
hestia server smp status         # the record merged with live process state
hestia server smp stop           # stop the running server
hestia server smp restart        # stop, then start again
hestia server smp rename cozy    # rename (stopped): rewrites the display name;
                                 #   the id, directory, ports and data stay put
hestia server smp remove         # delete the server (its jar, world and all)
hestia server smp mod add <slug> # servers take mods (fabric/plugin flavors)
hestia server smp datapack add <slug>   # datapacks install into the server's world
hestia server smp datapack add --file ./pack.zip   # any kind imports a local file
```

## Instances

Clients; files materialise at first launch. Same shape: catalogue verbs take no
entry, the rest are entry-first.

```bash
hestia instance create           # bare: the fullscreen wizard — flavor →
                                 #   version → name → memory → confirm; any
                                 #   argument switches to the flag-driven flow
hestia instance create fabric 1.21.1 -n modded --memory 4G
hestia instance list             # managed instances and their state
hestia instance versions [flavor] # game versions a flavor offers
hestia instance flavors          # the available flavors
hestia instance modded launch    # ensures java/client/libraries/assets, runs,
                                 #   then follows the logs fullscreen
                                 #   (-d/--detach returns immediately)
hestia instance modded launch --new-session   # launch another session while one
                                 #   is already running (default refuses — see
                                 #   "Multiple sessions" below)
hestia instance modded update 1.21.4  # move to another version (saves stay,
                                 #   but nothing is backed up — instances
                                 #   have no backups; files download at the
                                 #   next launch; a downgrade asks for a
                                 #   confirm)
hestia instance modded config set jvm-args "-XX:+UseG1GC"  # memory / jvm-args
hestia instance modded logs -n 50 # captured output — the newest running session
                                 #   (-f opens the fullscreen log session; piped
                                 #   it streams plainly)
hestia instance modded info      # the record and each running session (handles)
hestia instance modded stop      # kill every session (--session <h> targets one)
hestia instance modded restart   # stop, then launch again (--session <h> for one)
hestia instance modded rename mp # rename (stopped): rewrites the display name;
                                 #   the id, directory and saves stay put
hestia instance modded remove    # delete the instance (its saves and all)
```

### Multiple sessions

An instance can run **more than one session at a time**, but it is off by
default: `launch`/`play` refuse an instance that is already running unless you
pass `--new-session`.

```bash
hestia play modded               # session 1
hestia play modded               # refused: "already running; pass --new-session"
hestia play modded --new-session # session 2, running alongside session 1
hestia instance modded info      # lists each session with its handle, pid, state
```

Each session writes its own log (`<instance>/logs/session-N.log`), so their
output never interleaves. By default `logs` targets the newest running session
and `stop` stops **all** of the instance's sessions — target one with
`--session <handle>` (the handle is the short number `info` shows, or the full
process id):

```bash
hestia instance modded stop --session 1      # stop just session 1
hestia instance modded logs --session 2 -f   # follow session 2's output
hestia instance modded restart --session 1   # replace session 1 (others keep running)

hestia stop modded --session 1               # the shortcuts take it too
hestia logs modded --session 2 -f
```

Sessions share one `data/`, so Minecraft's own `session.lock` arbitrates a
singleplayer world (a second session can't open the same world). Servers stay
single — a world has one writer, so `--session` is an instance-only flag.

### Content on an instance

Mods, resource packs, shaders, and datapacks install per entry. Every kind
takes a project slug/id, a source page URL, or a local `--file` — or, with no
item on a terminal, opens the **fullscreen install session**: a boxed search
bar over live results with a detail pane, space checks any number of items,
`v` pins a version, Enter reviews the batch, and one confirm installs them all
as a single job (failures report per item; the rest proceed):

```bash
hestia instance modded mod add   # fullscreen search → select → review → install
hestia instance modded mod add sodium      # install a mod (resolves required
                                 #   deps; --version pins one; the file is
                                 #   mirrored into the game dir at launch)
hestia instance modded mod add https://modrinth.com/mod/lithium  # a page URL
hestia instance modded mod add --file ./my-mod.jar   # import a local file
hestia instance modded mod list  # installed mods (+ any untracked jars in the
                                 #   game dir)
hestia instance modded mod update [sodium]   # newest compatible (all, or one)
hestia instance modded mod remove sodium
hestia instance modded resourcepack add <slug>   # same verbs for packs/shaders
hestia instance modded shader add <slug>
```

Datapacks load from inside a save world, so an instance datapack names the
world(s) it goes into. Run `datapack add` with no arguments for the fullscreen
session — search and check the datapacks, and the review step picks the
world(s) (`w` reopens the picker; space toggles, enter confirms). For scripts,
pass the slug and a repeatable `--world`. The same datapack can live in
several worlds at once:

```bash
hestia instance modded datapack add                # 1) search a datapack  2) select world(s)
hestia instance modded datapack add terralith --world Alpha --world Beta
hestia instance modded datapack add --file ./pack.zip --world Alpha
hestia instance modded datapack list      # installed datapacks, with their world
hestia instance modded datapack remove terralith   # removes it from every world
hestia instance modded datapack remove terralith --world Alpha   # only that world's copy
hestia instance modded datapack update [item]      # updates it in each world
```

Worlds are shared across instances (linked `saves/` — see below), so a
datapack installed into a world is active for every instance that opens that
world. Only the installing instance tracks it; the others list it as
untracked world data.

## Shortcuts

One verb resolves a name across servers and instances, so you need not recall
which kind it is (a name matching both asks you to qualify it).

```bash
hestia play                      # launch an instance — one runs directly, several
                                 #   prompt a pick; follows the logs (-d skips).
                                 #   --new-session runs another alongside a
                                 #   running one (default refuses)
hestia start modded              # start a server (attaches its console) or launch
                                 #   an instance (follows its logs); -d/--detach
                                 #   returns immediately
hestia stop modded               # stop whichever it is (all sessions, for an instance)
hestia restart modded            # restart whichever it is (attaches like start)
hestia logs modded -f            # follow its captured output fullscreen
```

## Content discovery

Modrinth today; installs are per-entry (above).

```bash
hestia mod search                # bare, on a terminal: the fullscreen browser
                                 #   (type to search, detail pane, Enter shows
                                 #   versions); filters below seed it
hestia search sodium             # with a query: prints results (alias for
                                 #   `mod search`)
hestia mod search sodium -l fabric -g 1.21.1   # filter by loader / version
hestia modpack search "create"   # browse other kinds: modpack, resourcepack,
hestia resourcepack search faithful            #   shader, datapack
hestia mod info sodium           # a project's details (downloads, sides, …)
hestia mod versions sodium -l fabric -g 1.21.1  # downloadable versions
hestia sources                   # the available content sources
```

## Download cache

```bash
hestia cache info                # size and entry count
hestia cache list                # cached blobs
hestia cache clear               # evict everything
```

## Shared settings/configs

Settings, configs — and worlds — are shared across your instances
automatically. **File** targets (`options.txt`, key-merged with pack
selection kept per-instance; `servers.dat`) are copied: each instance keeps
its own copy, reconciled newest-wins at every launch. **Folder** targets
(`saves`, `config`, `screenshots`) are **linked** into the shared store (a
symlink on Linux/macOS, a junction on Windows): every instance opens the
same physical folders, so a world exists once and appears everywhere
instantly. It works out of the box — no setup.

A folder is only linked when it is missing, empty, or already linked — an
existing non-empty folder (a pre-linking instance's `saves/`, say) is never
touched. `sync status` shows it as *cannot link*; move its contents into the
store with `adopt`:

```bash
hestia sync status               # store path, targets, per-instance link state

hestia instance modded sync adopt        # move existing folders into the store
hestia instance modded sync adopt saves  # …or just one target
```

Adopt is all-or-nothing per folder: a name that already exists in the store
(two instances with a world called `New World`) refuses that folder and
lists the collisions — rename one, then retry. Nothing is ever merged or
overwritten.

Sync is **instance-only**: a server's configuration is per-server
infrastructure, managed through `server <name> config …` and
`server.properties`, and is never shared.

```bash
hestia sync add screenshots --folder   # share a folder (linked)
hestia sync add optionsof.txt          # share a file (copied)
hestia sync remove servers.dat         # keep each instance's list local
```

Paths are **game-relative** (relative to `data/`). `..` escapes and the
launcher-managed content directories (`mods`, `resourcepacks`, `shaderpacks`)
are rejected — the content system already shares content. `saves` can only be
shared as a folder (linked), never copied.

Two things to know about shared worlds: opening the same world from two
instances at once is only guarded by Minecraft's own `session.lock`, and
instances on different versions or loaders writing one world can corrupt it.
And until instance import/export lands, instance data — the shared worlds
store included — has **no backup story**; keep your own copies of worlds you
care about.

## Configuration

Typed settings, stored as JSON.

```bash
hestia config get <key>          # read a setting
hestia config set <key> <value>  # write a setting
hestia config list               # every setting
hestia config get home           # resolved data directory
hestia config set home <dir>     # persist the data dir (empty reverts to default)
hestia config get autostart      # true if the daemon starts at login
hestia config set autostart true # register the daemon to start at login
```

The data directory is resolved as: `--home` → `$HESTIA_HOME` → a persisted
pointer (`config set home`) → the platform default (`~/.hestia`, or
`%APPDATA%\Hestia` on Windows). **Debug builds** anchor the default at
`<workspace>/.hestia` so development never populates the real per-user directory.

## Daemon lifecycle

Servers and instances keep running across daemon stops/restarts and are
re-adopted by the next daemon.

```bash
hestia daemon status             # is the daemon running, and what is it supervising
hestia daemon start              # start it
hestia daemon restart            # restart it (workloads survive)
hestia daemon stop               # asks about running workloads (piped: --all/--keep)
hestia daemon stop --all         # stop supervised processes too
hestia daemon stop --keep        # leave them running (script-safe)
```

## Global flags

Accepted in any position.

```bash
hestia -v java list              # verbose / debug logging (-vv for trace);
                                 #   diagnostics also land in logs/hestia.log
hestia -q java list              # errors only on the console
hestia --home /path/to/dir config get home
hestia --version
```
