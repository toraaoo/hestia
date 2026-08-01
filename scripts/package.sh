#!/usr/bin/env bash
# Build release artifacts locally, mirroring the release workflow.
#
#   scripts/package.sh            # sidecars + Tauri installers + portable archive
#   scripts/package.sh bundle     # Tauri installers only (deb/rpm/appimage or nsis)
#   scripts/package.sh portable   # portable archive only (.tar.gz on Linux, .zip on Windows)
#
# Tauri bundles the desktop app + the hestiad/hestia-tray/hestia sidecars into each
# installer. The portable archive is the same four binaries, but compiled with
# the `portable` feature, so they are a separate build rather than a copy of the
# installers' — see `portable()`.
. "$(dirname "$0")/lib/common.sh"

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
    windows) targets="nsis" ;;
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
  local ext="" name stage out
  [ "$os" = windows ] && ext=".exe"
  name="hestia-$version-$triple"
  stage="target/package/$name"
  # Its own target dir: a portable binary resolves a different data home, so it
  # must never end up in target/release/ where sidecars.sh stages the ones the
  # installers bundle.
  out="target/portable"

  # tauri-build asserts the externalBin files exist, and generate_context!
  # embeds frontend/dist — both are needed to compile the shell at all, even
  # though a plain `cargo build` bundles neither.
  scripts/sidecars.sh
  [ -d frontend/dist ] || (cd frontend && bun run build)

  log "building portable binaries"
  cargo build --release --target-dir "$out" \
    -p cli -p daemon -p tray -p desktop \
    --features cli/portable,daemon/portable,tray/portable,desktop/portable

  rm -rf "$stage"
  mkdir -p "$stage/bin" "$stage/data"
  # The split the Windows installer lays down, under the names common::app's
  # DESKTOP_BIN and TRAY_BIN put first.
  if [ "$os" = windows ]; then
    cp "$out/release/hestia-desktop.exe" "$stage/Hestia.exe"
    cp "$out/release/hestia-tray.exe" "$stage/Hestia Tray.exe"
  else
    cp "$out/release/hestia-desktop" "$out/release/hestia-tray" "$stage/"
  fi
  for bin in hestia hestiad; do
    cp "$out/release/$bin$ext" "$stage/bin/"
  done
  cp LICENSE README.md "$stage/"
  if [ "$os" = windows ]; then
    powershell -NoProfile -Command \
      "Compress-Archive -Path 'target/package/$name/*' -DestinationPath 'target/package/$name.zip' -Force"
    log "wrote target/package/$name.zip"
  else
    tar -C target/package -czf "target/package/$name.tar.gz" "$name"
    log "wrote target/package/$name.tar.gz"
  fi
}

case "$action" in
  all) bundle && portable ;;
  bundle) bundle ;;
  portable) portable ;;
  *) die "usage: $0 [all|bundle|portable]" ;;
esac
