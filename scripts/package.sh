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
  # Signatures are minisign, applied to the finished artifacts by
  # scripts/sign.sh — the bundler never signs anything.
  (cd crates/desktop && cargo tauri build --bundles "$targets")
  if [ "$os" = linux ]; then
    unbundle_host_libraries
  fi
}

# linuxdeploy's gtk plugin drags libwayland-client into the AppDir and AppRun
# puts it ahead of the host's. The host's Mesa then loads *that* copy, EGL
# init fails ("Could not create default EGL display: EGL_BAD_PARAMETER"), the
# web process aborts and the window comes up blank. It is on the AppImage
# project's excludelist for exactly this reason — a library the host graphics
# stack owns is never safe to ship — so drop it and repack.
unbundle_host_libraries() {
  local dir=target/release/bundle/appimage name
  local packer=linuxdeploy-plugin-appimage-x86_64.AppImage

  name="$(basename "$(find "$dir" -maxdepth 1 -name '*.AppImage' -print -quit)")"
  [ -n "$name" ] || die "no AppImage in $dir"
  [ -x "$dir/$packer" ] || die "$dir/$packer is missing; cannot repack"

  log "unbundling host libraries from $name"
  # APPIMAGE_EXTRACT_AND_RUN because CI runners have no FUSE, which is also
  # why the bundler itself sets it.
  (
    cd "$dir"
    rm -rf squashfs-root
    APPIMAGE_EXTRACT_AND_RUN=1 "./$name" --appimage-extract > /dev/null
    rm -f squashfs-root/usr/lib/libwayland-client.so.0
    APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 OUTPUT="$name" \
      "./$packer" --appdir squashfs-root > /dev/null
    rm -rf squashfs-root
  )
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
  # embeds frontend/dist — the shell serves it, so a stale one must not ship.
  scripts/sidecars.sh
  (cd frontend && bun run build)

  # Split like the installers (sidecars.sh, then cargo tauri build): one
  # invocation unifies the shell's features into the tray, which then imports
  # a comctl32 v6 entry point no manifest asks for and cannot load.
  log "building portable binaries"
  cargo build --release --target-dir "$out" \
    -p cli -p daemon -p tray \
    --features cli/portable,daemon/portable,tray/portable
  cargo build --release --target-dir "$out" \
    -p desktop --features desktop/portable,desktop/custom-protocol

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
