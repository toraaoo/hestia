#!/usr/bin/env bash
# Build the CLI, daemon, and tray as release binaries and stage them as Tauri
# sidecars, so `cargo tauri build` bundles them into every desktop installer.
#
# Tauri's externalBin requires each binary to carry the target-triple suffix
# (e.g. hestiad-x86_64-unknown-linux-gnu); the installer strips it on install.
#
#   scripts/sidecars.sh                 # host target
#   scripts/sidecars.sh <target-triple> # cross target (passed to cargo --target)
#   scripts/sidecars.sh --ensure        # no-op if the staged set already exists
set -euo pipefail
cd "$(dirname "$0")/.."

ensure=0
if [ "${1:-}" = "--ensure" ]; then
  ensure=1
  shift
fi

triple="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
[ -n "$triple" ] || { echo "could not determine target triple" >&2; exit 1; }

target_args=()
srcdir="target/release"
if [ -n "${1:-}" ]; then
  target_args=(--target "$triple")
  srcdir="target/$triple/release"
fi

ext=""
case "$triple" in
  *windows*) ext=".exe" ;;
esac

dest="crates/desktop/binaries"

if [ "$ensure" = 1 ]; then
  staged=1
  for bin in hestia hestiad tray; do
    [ -f "$dest/$bin-$triple$ext" ] || staged=0
  done
  if [ "$staged" = 1 ]; then
    exit 0
  fi
fi

echo "building sidecars for $triple"
cargo build --release "${target_args[@]}" -p cli -p daemon -p tray

mkdir -p "$dest"
for bin in hestia hestiad tray; do
  cp "$srcdir/$bin$ext" "$dest/$bin-$triple$ext"
  echo "  staged $dest/$bin-$triple$ext"
done
