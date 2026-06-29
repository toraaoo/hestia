#!/usr/bin/env bash
#
# build.sh — build targets, configuring the build dir lazily on first use.
#
#   build.sh [--release] [target...]
#
# No flag uses the fast dev build (Debug, desktop off). --release uses the full
# build/ (Release, desktop included). No target builds everything in that build.
# Targets accept friendly names (daemon, cli, tui, tray, desktop) or raw CMake
# target names. Safe to call from CI.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

case "${1:-}" in
    --release) shift; build_full Release "$@" ;;
    --dev)     shift; build_dev "$@" ;;
    *)         build_dev "$@" ;;
esac
