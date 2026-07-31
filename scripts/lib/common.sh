#!/usr/bin/env bash
# Sourced as the first line of every scripts/<verb>.sh: strict mode, the repo
# root as the working directory, and the one voice status output speaks in.
#
# The cd anchors on *this* file rather than the caller's, so a verb behaves the
# same whether it was run as scripts/build.sh, ./build.sh or an absolute path —
# every path in a script here is repo-relative and stays that way.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

if [ -t 1 ]; then _C='\033[1;36m'; _R='\033[0m'; else _C=''; _R=''; fi

log() { printf '%b==>%b %s\n' "$_C" "$_R" "$*"; }

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}
