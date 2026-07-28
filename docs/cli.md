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
overrides the data directory when `hestia daemon start` spawns the daemon.

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
hestia server flavors            # the available flavors, what each takes, and
                                 #   a line on what it is — all from the daemon
```

Everything that acts on one server is entry-first — `hestia server <name> <action>`:

```bash
hestia server smp config list    # memory, jvm-args, and server.properties keys
hestia server smp config set memory 4G          # applies from the next start
hestia server smp config set motd "hi"          # any server.properties key its
                                                #   version knows (validated
                                                #   against the derived schema)
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
                                 #   log session; piped it streams plainly).
                                 #   -f follows the *server*: it works on a
                                 #   stopped one and rides through a restart
hestia server smp status         # live process state + a running server's ping
hestia server smp info           # descriptor, on-disk folder, and disk footprint
hestia server smp stop           # stop the running server
hestia server smp restart        # stop, then start again
hestia server smp rename cozy    # rename (stopped): rewrites the display name;
                                 #   the id, directory, ports and data stay put
hestia server smp remove         # delete the server (its jar, world and all)
hestia server smp mod add <slug> # a modloader server (fabric) takes mods
hestia server smp plugin add <slug>     # a paper/folia/spigot/bukkit server takes plugins
hestia server smp datapack add <slug>   # datapacks install into the server's world
hestia server smp datapack add --file ./pack.zip   # any kind imports a local file
```

What a server takes depends on its flavor: mods on `fabric`/`neoforge`,
plugins on `paper`/`folia`/`spigot`/`bukkit`, datapacks on any of them, and
nothing else on `vanilla` — `server flavors` prints the set per flavor rather
than leaving you to remember it.
A `neoforge` create builds its game jar locally, so it takes a few
minutes and cannot derive a property schema (`server config set` accepts any
key on one).
Asking for the wrong kind is refused naming what that server does take.

`spigot` and `bukkit` are compiled on this machine at create — nobody may
redistribute either jar, so SpigotMC's BuildTools builds it here. Budget
several minutes and a few hundred megabytes for the first one, and have `git`
installed (the create refuses up front when it is missing; Windows needs
nothing, BuildTools brings its own). One build produces both, so a later
`spigot` or `bukkit` server on the same game version is immediate.

`paper` and `folia` publish many builds per game version, so a build number is
the flavor's *loader version* — `hestia server versions paper` lists the game
versions and `--loader-version <build>` pins one; omitted, create takes the
newest stable build. Both start with the JVM flags PaperMC recommends for that
version unless the entry or `defaults.jvm-args` sets its own; `server <name>
info` reports the effective flags and where they came from.

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
                                 #   it streams plainly). -f follows the
                                 #   *instance*, so it picks up the next launch
hestia instance modded info      # descriptor, folder, disk footprint, and each
                                 #   running session (handles)
hestia instance modded worlds    # the save worlds, read from each level.dat:
                                 #   in-game name, folder, version, mode, when
                                 #   it was last played, and its size
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
hestia instance modded logs --session 2 -f   # follow session 2's output (ends
                                             #   with that session, unlike the
                                             #   instance-wide follow)
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
hestia logs modded -f            # follow its captured output fullscreen (the
                                 #   entry is the subject: a stop is a line in
                                 #   the stream, not the end of it)
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
hestia resourcepack search faithful            #   shader, datapack, plugin
hestia plugin search luckperms   # server plugins (paper/folia/bukkit/spigot)
hestia mod info sodium           # a project's details (downloads, sides, …)
hestia mod versions sodium -l fabric -g 1.21.1  # downloadable versions
hestia sources                   # the available content sources
```

## Modpacks

A pack pins its own loader and game version, so installing one *builds* the
entry it wants. The one argument takes any of the three ways to name a pack —
an existing path is a `.mrpack`, a scheme is a URL, anything else is a project
on the source; omit it on a terminal and a searchable picker opens over live
search results.

```bash
hestia modpack install fabulously-optimized     # → a new instance
hestia modpack install                          # pick one interactively
hestia modpack install fabulously-optimized --name cozy
hestia modpack install ./pack.mrpack            # a local .mrpack
hestia modpack install https://modrinth.com/modpack/…/version/6.2.1
hestia modpack install <pack> --server --eula   # → a new server instead
hestia modpack install <pack> --into cozy       # into an existing entry
hestia instance create --modpack <pack>         # the same thing, noun-first
hestia server create --modpack <pack> --eula
```

The pack's mods become ordinary content — `instance <name> mod list` shows
them, and each can be updated on its own. What the entry does with the pack as
a whole is entry-first:

```bash
hestia instance cozy modpack status     # which pack, which version
hestia instance cozy modpack update     # → the newest published version
hestia instance cozy modpack update 6.3.0 --downgrade
hestia instance cozy modpack remove     # keeps files you have edited
hestia server smp modpack status        # servers, the same verbs
```

An update carries the entry's game version with it — that is what updating a
pack means — so a pack built for an older version needs `--downgrade`. Files
the pack wrote into the game directory that you have since edited are never
overwritten or deleted; the command says which it kept.

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

A folder that already holds an instance's own files is **adopted**: its
contents move into the store and the folder becomes a link, at the launch that
would otherwise have left it unshared. Nothing is ever merged or overwritten,
so the one thing that stops it is a name the store already has (two instances
with a world called `New World`) — `sync status` shows that folder as
*clashes with the store*, and it stays local until you rename or delete the
clashing files. Adopt on demand is the same migration:

```bash
hestia sync status               # sharing on/off, store path, targets, link state

hestia instance modded sync adopt        # move existing folders into the store
hestia instance modded sync adopt saves  # …or just one target
```

An instance running a **modpack** keeps its own `config/`: the pack ships that
tree, so it is not folded into what every other instance reads. Adopt it
explicitly if you want it shared anyway — the link is honoured from then on.

Sharing can be switched off entirely; folders already linked stay linked.

```bash
hestia config set sync.enabled false    # every instance keeps its own settings
hestia config get sync.enabled
```

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
hestia config set sync.enabled false          # stop sharing settings across instances
hestia config set announcements.enabled false # stop fetching news and notices
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

Underneath the entry verbs sits the supervisor's own view — every workload the
daemon is running, across servers and instances, keyed the way the supervisor
keys them. Reach for it when the entry-scoped verbs cannot answer: a server's
`status` cannot show you an instance session, and neither shows a process whose
entry was removed out from under it.

```bash
hestia process list              # everything supervised, with state and pid
hestia process status server-<id>   # one process (exits 3 when not running)
hestia process logs instance-<id>_1 -n 50   # its captured output
hestia process stop server-<id>  # SIGTERM, then a hard kill after the grace
```

A plain `hestia daemon stop` with a server or instance running does **not**
guess: "stop the launcher" says nothing about the server, so it prompts on a
terminal and, when piped, exits non-zero naming both flags — a script has to
state which it meant. With nothing running there is nothing to ask and it stops
immediately. The tray's **Quit** and the desktop's stop button always mean
`--keep`: neither can ask, and neither should end a running server on your
behalf.

## News and notices

Announcements about Hestia itself, fetched from its published feed. Every entry
is filtered against *this* build — an entry can name a platform, a release
channel and a version range — so what you see is what applies to you.

```bash
hestia news                      # what applies to this build and is unread
hestia news list --all           # including what you've already read
hestia news show <id>            # one announcement in full
hestia news read                 # mark every unread one read
hestia news read <id>...         # mark specific ones read
hestia news refresh              # check now instead of waiting for the poll
```

Reads answer from the daemon's cache, so they are instant and work offline;
`refresh` is the only verb that touches the network. The daemon polls every six
hours, and read state is shared with the desktop — marking something read in
one is marking it read in both.

The feed is the daemon's only unprompted outbound request. Turn it off with:

```bash
hestia config set announcements.enabled false
```

Nothing is fetched while it is off, and `hestia news` says so rather than
pretending the feed is empty.

## Updating Hestia

```bash
hestia self-update               # check, confirm, download and apply
hestia self-update --yes         # no confirmation prompt
```

The installer is verified against a compiled-in signing key before it runs; an
artifact that fails to verify is discarded rather than applied. The desktop has
its own updater under Settings (it can replace a running binary and restart
into it, which the CLI path cannot).

## Global flags

Accepted in any position.

```bash
hestia -v java list              # debug logging: what the command is doing
hestia -vv java list             # …plus the wire: every frame this process sent
                                 #   and received, with its channel, correlation
                                 #   id, size and round-trip time
hestia -q java list              # errors only on the console
hestia --home /path/to/dir config get home
hestia --version
```

Diagnostics also land in `logs/hestia.log` regardless of the console level.
`-vv` is specifically for a CLI-versus-daemon disagreement: it shows the
conversation from this side, which the daemon's own logs cannot. Payloads are
never logged — they carry access tokens and RCON passwords — so a frame reports
its size, not its contents.

## Exit codes

A state query has two honest answers, and a shell reads only the exit code — so
"the daemon is stopped" must not look like "the daemon is running", nor like
"I could not tell". The vocabulary is systemd's:

| code | meaning                                                             |
|------|---------------------------------------------------------------------|
| `0`  | the command did what was asked; a state query found the subject running |
| `3`  | the query was answered and the subject is **not** running            |
| `1`  | the command failed — no daemon, invalid input, or the operation errored |
| `2`  | usage error (an unknown flag or subcommand)                          |

```bash
if hestia daemon status >/dev/null; then echo up; fi   # true only when running
hestia server smp status >/dev/null; case $? in
  0) echo running ;;
  3) echo stopped ;;
  *) echo "could not ask" ;;
esac
```

Only verbs that assert whether one subject is running use `3` — `daemon status`
and `server <name> status`. Verbs that *describe* rather than assert (`info`,
`sync status`, every `list`) always exit `0`: "inactive" is not a claim they
make.
