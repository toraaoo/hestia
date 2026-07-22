#!/usr/bin/env bash
# Build then run a binary. Usage:
#   scripts/run.sh cli [args...]         # the hestia CLI
#   scripts/run.sh daemon [args...]      # hestiad
#   scripts/run.sh desktop               # the Tauri shell against the Vite dev server
#   scripts/run.sh --release daemon serve   # run the release build instead of debug
set -euo pipefail
cd "$(dirname "$0")/.."

profile=""
rest=()
for arg in "$@"; do
  case "$arg" in
    --release) profile="--release" ;;
    *) rest+=("$arg") ;;
  esac
done
set -- "${rest[@]}"

target="${1:-cli}"
shift || true

case "$target" in
  cli)     cargo run $profile -p cli -- "$@" ;;
  daemon)  cargo build $profile -p tray
           cargo run $profile -p daemon -- "$@" ;;
  desktop)
    if [ -n "$profile" ]; then
      # A real release run needs the prod frontend (Tauri builds it) and the
      # staged sidecars; launch the built binary rather than the Vite dev server.
      scripts/sidecars.sh
      (cd crates/desktop && cargo tauri build --no-bundle)
      exec ./target/release/hestia-desktop
    else
      (cd crates/desktop && cargo tauri dev)
    fi
    ;;
  *) echo "usage: $0 [--release] [cli|daemon|desktop] [args]" >&2; exit 1 ;;
esac
