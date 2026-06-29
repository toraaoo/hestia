#!/usr/bin/env bash
#
# run.sh — build a binary, then run it.
#
#   run.sh <daemon|cli|tray> [args...]   build from the dev profile, then run
#   run.sh desktop [args...]             desktop with hot-reload (HMR)
#
# The desktop case builds the full build/ in Debug, starts the Vite dev server,
# and launches the app pointed at it, so frontend edits hot-reload. Override the
# URL with DEV_URL (default http://localhost:5173). Stopping the app also stops
# the dev server.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

run_desktop_hmr() {
    build_full Debug desktop
    local bin="$FULL_DIR/Debug/Hestia"
    local url="${DEV_URL:-http://localhost:5173}"

    log "Starting Vite dev server (HMR)"
    ( cd "$ROOT/apps/desktop/frontend" && bun install && bun run dev ) &
    local vite_pid=$!
    trap 'kill "$vite_pid" 2>/dev/null || true' EXIT INT TERM

    if command -v curl >/dev/null 2>&1; then
        log "Waiting for $url ..."
        for _ in $(seq 1 60); do
            curl -fsS -o /dev/null "$url" 2>/dev/null && break
            sleep 0.5
        done
    else
        sleep 3
    fi

    log "Launching $bin --dev-url=$url"
    "$bin" --dev-url="$url" "$@"
}

[ "$#" -ge 1 ] || die "usage: run.sh <daemon|cli|tray|desktop> [args...]"
name="$1"; shift

case "$name" in
    desktop|Hestia)
        run_desktop_hmr "$@" ;;
    *)
        bin="$(binary_for "$name")"
        [ -n "$bin" ] || die "run: unknown target '$name' (daemon, cli, tray, desktop)"
        build_dev "$name"
        path="$DEV_DIR/Debug/$bin"
        log "Running $path $*"
        exec "$path" "$@" ;;
esac
