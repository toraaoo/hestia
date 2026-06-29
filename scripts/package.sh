#!/usr/bin/env bash
#
# package.sh — build the release artifacts locally.
#
#   package.sh
#
# Builds the full Release tree, then produces: the flat portable archive
# (cmake/package_portable.cmake), the distro packages (CPack: DEB/RPM), and the
# AppImage when linuxdeploy + appimagetool are on PATH. Artifacts land in build/.
# Mirrors the release workflow so it doubles as a CI step.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

build_full Release

log "Packaging portable archive -> $FULL_DIR"
cmake -DBUILD_DIR="$FULL_DIR" -DOUT_DIR="$FULL_DIR" -DSOURCE_DIR="$ROOT" \
      -P "$ROOT/cmake/package_portable.cmake"

log "Packaging distro packages (CPack)"
( cd "$FULL_DIR" && cpack )

if [ "$(uname -s)" = "Linux" ] && command -v linuxdeploy >/dev/null 2>&1 \
   && command -v appimagetool >/dev/null 2>&1; then
    log "Building AppImage"
    APPIMAGE_EXTRACT_AND_RUN=1 "$ROOT/packaging/appimage/build-appimage.sh" "$FULL_DIR" "$FULL_DIR"
fi
