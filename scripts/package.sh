#!/usr/bin/env bash
# Build release artifacts locally.
#   scripts/package.sh cli       # cli/daemon/tray archives + installers via cargo-dist
#   scripts/package.sh desktop   # the Tauri bundle (.deb/.rpm/.AppImage/.msi/.dmg)
set -euo pipefail
cd "$(dirname "$0")/.."

target="${1:-cli}"
shift || true

case "$target" in
  cli|dist) cargo dist build "$@" ;;
  desktop)  (cd crates/desktop && cargo tauri build "$@") ;;
  *) echo "usage: $0 [cli|desktop] [flags]" >&2; exit 1 ;;
esac
