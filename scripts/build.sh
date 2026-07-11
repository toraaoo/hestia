#!/usr/bin/env bash
# Thin cargo wrapper. Usage:
#   scripts/build.sh [cli|daemon|desktop|all] [extra cargo flags...]
# Examples:
#   scripts/build.sh cli --release
#   scripts/build.sh all
set -euo pipefail
cd "$(dirname "$0")/.."

target="${1:-all}"
shift || true

# A running hestiad holds the previous build in memory; stop every instance so
# the next client auto-spawns the freshly built daemon. SIGTERM is the daemon's
# graceful stop, so any supervised workloads keep running. This is the POSIX
# path — on Windows, win.ps1 stops the daemon natively before invoking us (Git
# Bash has no pkill, and a live hestiad.exe would hard-lock the overwrite).
stop_hestia() {
  pkill -TERM -x hestiad 2>/dev/null || true
  for _ in {1..20}; do
    command -v pgrep >/dev/null 2>&1 || return 0
    pgrep -x hestiad >/dev/null 2>&1 || return 0
    sleep 0.25
  done
}
stop_hestia

# Compiling the desktop crate requires the Tauri externalBin sidecars to be
# staged, or its build script fails on the missing resource paths.
case "$target" in
  cli)     cargo build -p cli "$@" ;;
  daemon)  cargo build -p daemon "$@" ;;
  desktop) scripts/sidecars.sh --ensure
           (cd crates/desktop && cargo tauri build "$@") ;;
  all)     scripts/sidecars.sh --ensure
           cargo build --workspace "$@" ;;
  *) echo "usage: $0 [cli|daemon|desktop|all] [cargo flags]" >&2; exit 1 ;;
esac
