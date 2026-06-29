# lib.sh — shared helpers for the scripts in this directory.
#
# Sourced, not executed. Defines the configure/build primitives the entry
# scripts (build.sh, run.sh, …) sit on top of.
#
# Two build directories:
#   build-dev/   Debug, desktop OFF  — fast core iteration (no CEF)
#   build/       the full build      — Release, or Debug for desktop HMR
#
# build/ holds one configuration at a time (like the README): a release build
# configures it Release; desktop HMR configures it Debug. Switching reconfigures.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEV_DIR="$ROOT/build-dev"
FULL_DIR="$ROOT/build"

# Prefer Ninja; fall back to the platform default generator.
GEN_ARGS=()
command -v ninja >/dev/null 2>&1 && GEN_ARGS=(-G Ninja)

# Colours only on a terminal, so CI logs stay clean.
if [ -t 1 ]; then _C='\033[1;36m'; _R='\033[0m'; else _C=''; _R=''; fi
log() { printf '%b==>%b %s\n' "$_C" "$_R" "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

# Friendly name -> CMake target. Unknown names pass through unchanged.
target_for() {
    case "$1" in
        daemon|hestiad)  echo hestia_daemon ;;
        cli|hestia)      echo hestia_cli ;;
        tui)             echo hestia_tui ;;
        tray)            echo hestia_tray ;;
        desktop|Hestia)  echo hestia_desktop ;;
        *)               echo "$1" ;;
    esac
}

# Friendly name -> built binary file name (empty if it has no runnable binary).
binary_for() {
    case "$1" in
        daemon|hestiad|hestia_daemon) echo hestiad ;;
        cli|hestia|hestia_cli)        echo hestia ;;
        tray|hestia_tray)             echo hestia-tray ;;
        desktop|Hestia|hestia_desktop) echo Hestia ;;
        *) echo "" ;;
    esac
}

build_frontend() {
    log "Building desktop frontend (bun)"
    ( cd "$ROOT/apps/desktop/frontend" && bun install && bun run build )
}

# CMakeRC embeds dist/ at configure time, so it must exist before configuring the
# full build — even in Debug (the dev server overrides it at runtime).
ensure_frontend() { [ -d "$ROOT/apps/desktop/frontend/dist" ] || build_frontend; }

configure_dev() {
    log "Configuring dev (Debug, desktop off) -> $DEV_DIR"
    cmake -S "$ROOT" -B "$DEV_DIR" "${GEN_ARGS[@]}" \
        -DCMAKE_BUILD_TYPE=Debug -DBUILD_DESKTOP=OFF "$@"
}
ensure_dev() { [ -f "$DEV_DIR/CMakeCache.txt" ] || configure_dev; }

# configure_full <Debug|Release> [extra cmake args]
configure_full() {
    local type="$1"; shift
    ensure_frontend
    log "Configuring full build ($type) -> $FULL_DIR"
    cmake -S "$ROOT" -B "$FULL_DIR" "${GEN_ARGS[@]}" \
        -DCMAKE_BUILD_TYPE="$type" "$@"
}

_full_build_type() {
    [ -f "$FULL_DIR/CMakeCache.txt" ] || return 1
    grep -m1 '^CMAKE_BUILD_TYPE:' "$FULL_DIR/CMakeCache.txt" | cut -d= -f2-
}
# Reconfigure only when build/ isn't already the requested type.
ensure_full() { [ "$(_full_build_type || true)" = "$1" ] || configure_full "$1"; }

# _run_build <dir> <label> [target...]
_run_build() {
    local dir="$1" label="$2"; shift 2
    if [ "$#" -eq 0 ]; then
        log "Building all targets ($label)"
        cmake --build "$dir"
    else
        local args=() t
        for t in "$@"; do args+=(--target "$(target_for "$t")"); done
        log "Building ($label): $*"
        cmake --build "$dir" "${args[@]}"
    fi
}

build_dev() { ensure_dev; _run_build "$DEV_DIR" dev "$@"; }
# build_full <Debug|Release> [target...]
build_full() { local type="$1"; shift; ensure_full "$type"; _run_build "$FULL_DIR" "$type" "$@"; }
