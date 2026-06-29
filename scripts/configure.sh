#!/usr/bin/env bash
#
# configure.sh — configure a build directory.
#
#   configure.sh [--release] [-- <extra cmake args>]
#
# No flag configures the fast dev build (Debug, desktop off). --release
# configures the full build/ (Release). Extra args after `--` are forwarded to
# cmake. Safe to call from CI.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

release=0
case "${1:-}" in
    --release) release=1; shift ;;
    --dev)     shift ;;
esac
[ "${1:-}" = "--" ] && shift

if [ "$release" = 1 ]; then
    configure_full Release "$@"
else
    configure_dev "$@"
fi
