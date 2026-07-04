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

case "$target" in
  cli)     cargo build -p cli "$@" ;;
  daemon)  cargo build -p daemon "$@" ;;
  desktop) (cd crates/desktop && cargo tauri build "$@") ;;
  all)     cargo build --workspace "$@" ;;
  *) echo "usage: $0 [cli|daemon|desktop|all] [cargo flags]" >&2; exit 1 ;;
esac
