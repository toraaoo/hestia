#!/usr/bin/env bash
# Regenerate every shipped icon from the masters in assets/icons/.
#
# assets/icons/ember.svg is the single source of truth for the app mark; every
# raster below is derived from it and checked in only because the build tools
# (Tauri's bundler, the tray's include_bytes!, Vite) need files, not vectors.
# Edit the SVG, run this, commit the result — never hand-edit the outputs.
#
#   crates/desktop/icons/*        Tauri bundle icons (all platforms)
#   crates/tray/assets/icon.png   the bare mark, transparent, for the tray
#   frontend/public/favicon.ico   the webview tab icon
#
# Needs ImageMagick (`magick`) and tauri-cli (`cargo install tauri-cli`).
#
#   scripts/gen-icons.sh
set -euo pipefail
cd "$(dirname "$0")/.."

src="assets/icons/ember.svg"
[ -f "$src" ] || { echo "missing $src" >&2; exit 1; }
command -v magick >/dev/null || { echo "ImageMagick (magick) is required" >&2; exit 1; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Tauri's icon command reads rasters, not SVG, and wants a 1024px square.
magick -background none "$src" -resize 1024x1024 "$tmp/app.png"

echo "generating crates/desktop/icons"
cargo tauri icon "$tmp/app.png" -o crates/desktop/icons >/dev/null
# Desktop-only project; tauri emits mobile sets unconditionally.
rm -rf crates/desktop/icons/android crates/desktop/icons/ios

echo "generating frontend/public/favicon.ico"
magick -background none "$src" \
  -define icon:auto-resize=16,24,32,48,64,256 frontend/public/favicon.ico

# The tray sits on the panel's own background, so it takes the bare mark: no
# plate, trimmed to the cube and re-padded so every platform scales it alike.
echo "generating crates/tray/assets/icon.png"
grep -v '<rect' "$src" > "$tmp/mark.svg"
magick -background none "$tmp/mark.svg" -resize 1024x1024 \
  -trim +repage -resize 240x240 \
  -gravity center -background none -extent 256x256 \
  crates/tray/assets/icon.png

echo "done — commit the regenerated icons"
