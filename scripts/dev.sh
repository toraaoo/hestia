#!/usr/bin/env bash
#
# dev.sh — a terminal-first dev shell for the CLI + daemon.
#
#   dev.sh                    build daemon + CLI (dev), open a subshell with
#                             `hestia`/`hestiad` on PATH
#   dev.sh <hestia-args...>   build, then run `hestia <args...>` once
#   dev.sh --desktop [args]   desktop launcher with frontend hot-reload (HMR)
#
# The CLI auto-spawns the sibling daemon, so `hestia java list` just works.
# Debug builds keep data under <repo>/.hestia, so this never touches ~/.hestia.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

if [ "${1:-}" = "--desktop" ]; then
    shift
    run_desktop_hmr "$@"
    exit $?
fi

build_dev daemon cli
bindir="$DEV_DIR/Debug"

# One-shot: `dev.sh java list` runs the CLI once and exits.
if [ "$#" -gt 0 ]; then
    exec "$bindir/hestia" "$@"
fi

log "hestia + hestiad on PATH ($bindir). Ctrl-D / 'exit' to leave."
trap '"$bindir/hestia" daemon stop >/dev/null 2>&1 || true' EXIT
PATH="$bindir:$PATH" HESTIA_DEV=1 exec "${SHELL:-bash}" -i
