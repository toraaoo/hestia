#!/usr/bin/env bash
#
# clean.sh — remove build directories.
#
#   clean.sh [dev|release|all]   (default: dev)
#
# `release` removes the full build/ dir (also used for desktop HMR).

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

case "${1:-dev}" in
    dev)         log "Removing $DEV_DIR"; rm -rf "$DEV_DIR" ;;
    release)     log "Removing $FULL_DIR"; rm -rf "$FULL_DIR" ;;
    all)         log "Removing $DEV_DIR and $FULL_DIR"; rm -rf "$DEV_DIR" "$FULL_DIR" ;;
    *) die "usage: clean.sh [dev|release|all]" ;;
esac
