#!/usr/bin/env bash
#
# package.sh — build the release artifacts locally with CPack.
#
#   package.sh [cpack-generators]
#
# Builds the full Release tree, then packs it. With no argument CPack uses the
# platform defaults (Linux: TGZ;DEB;RPM, Windows: ZIP;NSIS;WIX) and, on Linux,
# also builds the AppImage if linuxdeploy + appimagetool are on PATH. Packages
# land in build/. Mirrors the release workflow so it doubles as a CI step.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

build_full Release

log "Packaging -> $FULL_DIR"
if [ -n "${1:-}" ]; then
    ( cd "$FULL_DIR" && cpack -G "$1" )
else
    ( cd "$FULL_DIR" && cpack )
    if [ "$(uname -s)" = "Linux" ] && command -v linuxdeploy >/dev/null 2>&1 \
       && command -v appimagetool >/dev/null 2>&1; then
        log "Building AppImage"
        APPIMAGE_EXTRACT_AND_RUN=1 "$ROOT/packaging/appimage/build-appimage.sh" "$FULL_DIR" "$FULL_DIR"
    fi
fi
