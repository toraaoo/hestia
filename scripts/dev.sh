#!/usr/bin/env bash
#
# dev.sh — a terminal-first dev shell for the CLI + daemon.
#
#   scripts/dev.sh                  build daemon + CLI (debug), open a subshell
#                                   with `hestia`/`hestiad` on PATH
#   scripts/dev.sh <hestia-args>    build, then run `hestia <args>` once
#   scripts/dev.sh --desktop [args] the Tauri desktop shell with frontend HMR
#
# The CLI auto-spawns the sibling daemon, so `hestia java list` just works.
# Debug builds keep data under <repo>/.hestia, so this never touches ~/.hestia.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -t 1 ]; then _C='\033[1;36m'; _R='\033[0m'; else _C=''; _R=''; fi
log() { printf '%b==>%b %s\n' "$_C" "$_R" "$*"; }

# Desktop launcher with Vite HMR: Tauri drives the frontend dev server itself.
if [ "${1:-}" = "--desktop" ]; then
    shift
    log "Desktop shell with frontend HMR (cargo tauri dev)"
    scripts/sidecars.sh --ensure
    cd crates/desktop
    exec cargo tauri dev "$@"
fi

log "Building daemon + CLI (debug)"
cargo build -p daemon -p cli
bindir="$PWD/target/debug"

# One-shot: `dev.sh java list` runs the CLI once and exits.
if [ "$#" -gt 0 ]; then
    exec "$bindir/hestia" "$@"
fi

log "hestia + hestiad on PATH ($bindir). Ctrl-D / 'exit' to leave."
trap '"$bindir/hestia" daemon stop >/dev/null 2>&1 || true' EXIT
PATH="$bindir:$PATH" HESTIA_DEV=1 exec "${SHELL:-bash}" -i
