#!/usr/bin/env bash
# Build release artifacts locally, mirroring the release workflow.
#
#   scripts/package.sh            # sidecars + Tauri installers + portable archive
#   scripts/package.sh bundle     # Tauri installers only (deb/rpm/appimage or nsis/msi)
#   scripts/package.sh portable   # portable archive only (.tar.gz on Linux, .zip on Windows)
#
# Tauri bundles the desktop app + the hestiad/tray/hestia sidecars into each
# installer; the portable archive is the same set of standalone binaries.
set -euo pipefail
cd "$(dirname "$0")/.."

action="${1:-all}"

triple="$(rustc -vV | sed -n 's/^host: //p')"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) os=windows ;;
  Darwin) os=macos ;;
  *) os=linux ;;
esac

bundle() {
  scripts/sidecars.sh
  case "$os" in
    windows) targets="nsis,msi" ;;
    macos) targets="app,dmg" ;;
    *) targets="deb,rpm,appimage" ;;
  esac
  # Updater artifacts (.sig) need the release signing key; without it in the
  # environment, build the plain installers so local packaging still works.
  config_args=()
  if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
    echo "TAURI_SIGNING_PRIVATE_KEY not set — skipping updater signatures" >&2
    config_args=(--config '{"bundle":{"createUpdaterArtifacts":false}}')
  fi
  (cd crates/desktop && cargo tauri build --bundles "$targets" "${config_args[@]}")
}

portable() {
  local ext="" name stage
  [ "$os" = windows ] && ext=".exe"
  name="hestia-$version-$triple"
  stage="target/package/$name"
  rm -rf "$stage"
  mkdir -p "$stage"
  for bin in hestia hestiad tray hestia-desktop; do
    [ -f "target/release/$bin$ext" ] && cp "target/release/$bin$ext" "$stage/"
  done
  cp LICENSE README.md "$stage/" 2>/dev/null || true
  if [ "$os" = windows ]; then
    powershell -NoProfile -Command \
      "Compress-Archive -Path 'target/package/$name/*' -DestinationPath 'target/package/$name.zip' -Force"
    echo "wrote target/package/$name.zip"
  else
    tar -C target/package -czf "target/package/$name.tar.gz" "$name"
    echo "wrote target/package/$name.tar.gz"
  fi
}

case "$action" in
  all) bundle && portable ;;
  bundle) bundle ;;
  portable) portable ;;
  *)
    echo "usage: $0 [all|bundle|portable]" >&2
    exit 1
    ;;
esac
