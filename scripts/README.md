# scripts

Helper scripts for local development and CI. They wrap the CMake incantations
from the [main README](../README.md#building) so you don't have to remember the
flags. Each script is a focused entry point; shared logic lives in `lib.sh`.

## Build directories

| Directory    | Configure                       | For                               |
|--------------|---------------------------------|-----------------------------------|
| `build-dev/` | Debug, **desktop off** (no CEF) | Fast daemon / CLI / TUI iteration |
| `build/`     | the full build                  | Release builds and desktop HMR    |

`build/` holds one configuration at a time, like the README: a release build
configures it `Release`; desktop HMR configures it `Debug`. Switching between the
two reconfigures it. The frontend (`dist/`) is built automatically whenever the
full build is configured.

## Scripts

| Script         | Usage                                     | Notes                                                 |
|----------------|-------------------------------------------|-------------------------------------------------------|
| `dev.sh`       | `dev.sh [hestia-args… \| --desktop …]`     | Terminal-first dev shell (CLI + daemon on PATH)       |
| `build.sh`     | `build.sh [--release] [target...]`        | No flag = dev. No target = all. **CI entry point**    |
| `configure.sh` | `configure.sh [--release] [-- cmake…]`    | Explicit configure; forwards extra cmake args. **CI** |
| `run.sh`       | `run.sh <daemon\|cli\|tray\|desktop> […]` | Builds then runs a single binary (desktop = no HMR)   |
| `clean.sh`     | `clean.sh [dev\|release\|all]`            | Remove build dir(s). Default: `dev`                   |
| `lib.sh`       | *(sourced, not run)*                      | Shared helpers and build primitives                   |

Target names accept friendly aliases (`daemon`, `cli`, `tui`, `tray`,
`desktop`) or raw CMake target names (`hestia_daemon`, …).

## Examples

```bash
scripts/dev.sh                   # build daemon + CLI, open a shell with them on PATH
scripts/dev.sh java list         # build, then run `hestia java list` once
scripts/dev.sh --desktop         # desktop app with frontend hot-reload
scripts/build.sh daemon          # quick rebuild of hestiad (dev)
scripts/build.sh                 # rebuild every dev target (no desktop)
scripts/run.sh cli java list     # build the CLI, then run `hestia java list`
scripts/run.sh desktop           # build + run the desktop app (embedded frontend, no HMR)
scripts/build.sh --release       # full release build, desktop included
scripts/clean.sh all             # wipe both build dirs
```

### Dev shell

`dev.sh` (no args) builds the daemon and CLI (fast dev profile, no CEF) and drops
you into an interactive subshell with `hestia`/`hestiad` on `PATH`. The CLI
auto-spawns the sibling daemon, so `hestia java list` just works; Debug builds
keep their data under `<repo>/.hestia`, so it never touches your real `~/.hestia`.
The daemon is stopped when you leave the shell. Pass `hestia` arguments to run a
single command instead of opening a shell.

### Desktop HMR

`dev.sh --desktop` builds the desktop app (configures `build/` as Debug), starts
the Vite dev server, and launches the app pointed at it, so frontend edits
hot-reload. Override the URL with `DEV_URL` (default `http://localhost:5173`);
stopping the app also stops the dev server.

## CI

The scripts are non-interactive, return proper exit codes, and suppress colour
when stdout isn't a terminal, so they double as CI steps:

```bash
scripts/configure.sh --release      # configure (also builds the frontend)
scripts/build.sh --release          # build
```

For a fast core-only lane that skips CEF entirely, drop the `--release`:
`scripts/build.sh` builds the daemon, CLI, and TUI in `build-dev/`.

Installing `ccache` speeds up rebuilds across both build dirs (picked up
automatically via the `USE_CCACHE` CMake option).
