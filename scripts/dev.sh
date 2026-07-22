#!/usr/bin/env bash
#
# dev.sh — a terminal-first dev shell for the CLI + daemon.
#
#   scripts/dev.sh                  build daemon, tray + CLI (debug), open a subshell
#                                   with `hestia`/`hestiad` on PATH
#   scripts/dev.sh <hestia-args>    build, then run `hestia <args>` once
#   scripts/dev.sh --release <hestia-args>  build release binaries, then run `hestia <args>` once
#
# The CLI auto-spawns the sibling daemon, so `hestia java list` just works.
# Debug builds keep data under <repo>/.hestia, so this never touches ~/.hestia.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -t 1 ]; then _C='\033[1;36m'; _R='\033[0m'; else _C=''; _R=''; fi
log() { printf '%b==>%b %s\n' "$_C" "$_R" "$*"; }

# Ignore an installed hestia entirely while developing: drop PATH entries that
# carry one, and pin a dev-only daemon endpoint so the dev CLI never reaches
# (or, via the exit trap, stops) an installed daemon.
strip_installed_hestia() {
  local kept="" dir
  local IFS=':'
  for dir in $PATH; do
    if [ -e "$dir/hestia" ] || [ -e "$dir/hestia.exe" ] ||
      [ -e "$dir/hestiad" ] || [ -e "$dir/hestiad.exe" ]; then
      log "ignoring installed hestia in $dir" >&2
      continue
    fi
    kept="${kept:+$kept:}$dir"
  done
  printf '%s' "$kept"
}
PATH="$(strip_installed_hestia)"
# On Windows the dev endpoint is a named pipe, supplied by win.ps1 before it
# forwards here.
export HESTIA_SOCK="${HESTIA_SOCK:-${XDG_RUNTIME_DIR:-/tmp}/hestiad-dev-$(id -u).sock}"

mode=debug
flags=()

if [ "${1:-}" = "--release" ]; then
  mode=release
  flags=(--release)
  shift
fi

log "Building daemon + tray + CLI ($mode)"
cargo build "${flags[@]}" -p daemon -p tray -p cli
bindir="$PWD/target/$mode"
# One-shot: `dev.sh java list` runs the CLI once and exits.
if [ "$#" -gt 0 ]; then
    exec "$bindir/hestia" "$@"
fi

log "hestia + hestiad on PATH ($bindir). Ctrl-D / 'exit' to leave."
trap '"$bindir/hestia" daemon stop >/dev/null 2>&1 || true' EXIT
PATH="$bindir:$PATH" HESTIA_DEV=1 exec "${SHELL:-bash}" -i
