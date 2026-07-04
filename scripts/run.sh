#!/usr/bin/env bash
#
# run.sh — build a binary, then run it.
#
#   run.sh <daemon|cli|tray> [args...]   build from the dev profile, then run
#   run.sh desktop [args...]             build the desktop app, then run it
#
# The desktop runs against its embedded frontend (no dev server). For frontend
# hot-reload (HMR) use `dev.sh --desktop`.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

[ "$#" -ge 1 ] || die "usage: run.sh <daemon|cli|tray|desktop> [args...]"
name="$1"; shift

case "$name" in
    desktop|Hestia|hestia_desktop)
        build_full Debug desktop
        path="$FULL_DIR/Debug/$(binary_for desktop)"
        log "Running $path $*"
        exec "$path" "$@" ;;
    *)
        bin="$(binary_for "$name")"
        [ -n "$bin" ] || die "run: unknown target '$name' (daemon, cli, tray, desktop)"
        build_dev "$name"
        path="$DEV_DIR/Debug/$bin"
        log "Running $path $*"
        exec "$path" "$@" ;;
esac
