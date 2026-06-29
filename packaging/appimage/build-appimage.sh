#!/usr/bin/env bash
#
# build-appimage.sh — package the desktop launcher (+ CLI/daemon) as an AppImage.
#
#   build-appimage.sh <build-dir> <output-dir>
#
# Assembles an AppDir from the installed "cli" and "desktop" components, bundles
# the desktop binary's shared-library dependencies with linuxdeploy, and packs
# the result with appimagetool. Requires `linuxdeploy` and `appimagetool` on PATH
# (the release workflow fetches both).
set -euo pipefail

BUILD_DIR=${1:?usage: build-appimage.sh <build-dir> <output-dir>}
OUT_DIR=${2:?usage: build-appimage.sh <build-dir> <output-dir>}
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)

WORK=$(mktemp -d)
APPDIR="$WORK/Hestia.AppDir"
mkdir -p "$APPDIR" "$OUT_DIR"

# The AppImage is the desktop bundle: daemon + tray + launcher, no standalone CLI.
cmake --install "$BUILD_DIR" --prefix "$APPDIR/usr" --component daemon
cmake --install "$BUILD_DIR" --prefix "$APPDIR/usr" --component desktop

# AppRun and desktop file live outside the AppDir so linuxdeploy installs them
# (passing a file already at AppDir/AppRun makes it copy onto itself and fail).
# CEF's SUID sandbox can't exist inside an AppImage mount, so the launcher runs
# with the sandbox disabled (deb/rpm keep the real SUID sandbox via postinst).
cat > "$WORK/AppRun" <<'EOF'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
export LD_LIBRARY_PATH="$HERE/usr/lib/hestia:${LD_LIBRARY_PATH:-}"
exec "$HERE/usr/lib/hestia/HestiaLauncher" --no-sandbox "$@"
EOF
chmod +x "$WORK/AppRun"

cat > "$WORK/hestia.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Hestia
GenericName=Minecraft Launcher
Exec=HestiaLauncher
Icon=hestia
Terminal=false
Categories=Game;
EOF

# Bundle dependencies of the desktop binary (linuxdeploy applies the AppImage
# library excludelist so glibc and friends stay on the host).
linuxdeploy --appdir "$APPDIR" \
    --executable "$APPDIR/usr/lib/hestia/HestiaLauncher" \
    --library "$APPDIR/usr/lib/hestia/libcef.so" \
    --desktop-file "$WORK/hestia.desktop" \
    --icon-file "$ROOT/packaging/icons/hestia.svg" \
    --custom-apprun "$WORK/AppRun"

appimagetool "$APPDIR" "$OUT_DIR/Hestia-$(uname -m).AppImage"
