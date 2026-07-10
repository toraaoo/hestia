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
