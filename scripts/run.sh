#!/usr/bin/env bash
# Build then run a binary. Usage:
#   scripts/run.sh cli [args...]         # the hestia CLI
#   scripts/run.sh daemon [args...]      # hestiad
#   scripts/run.sh desktop               # the Tauri shell against the Vite dev server
set -euo pipefail
cd "$(dirname "$0")/.."

target="${1:-cli}"
shift || true

case "$target" in
  cli)     cargo run -p cli -- "$@" ;;
  daemon)  cargo build -p tray
           cargo run -p daemon -- "$@" ;;
  desktop) (cd crates/desktop && cargo tauri dev) ;;
  *) echo "usage: $0 [cli|daemon|desktop] [args]" >&2; exit 1 ;;
esac
