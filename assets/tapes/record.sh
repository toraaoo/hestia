#!/usr/bin/env bash
# Render the CLI demo GIFs in assets/demo/.
#
# Each tape runs against ~/.hestia-demo, restored from ~/.hestia-demo-base
# before every take so a recording always starts from the same state. See
# README.md in this directory for how to build the base.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"

base="${HESTIA_DEMO_BASE:-$HOME/.hestia-demo-base}"
home="${HESTIA_DEMO_HOME:-$HOME/.hestia-demo}"
bin="$root/target/debug"

[[ -d $base ]] || { echo "no demo base at $base — see assets/tapes/README.md" >&2; exit 1; }
[[ -x $bin/hestia ]] || { echo "build first: cargo build -p cli -p daemon" >&2; exit 1; }
command -v vhs >/dev/null || { echo "vhs not on PATH" >&2; exit 1; }

reset() {
  HESTIA_HOME="$home" "$bin/hestiad" stop >/dev/null 2>&1 || true
  sleep 1
  rm -rf "$home"
  cp -r "$base" "$home"
  HESTIA_HOME="$home" "$bin/hestia" daemon start >/dev/null 2>&1
  sleep 2
}

tapes=("$@")
[[ ${#tapes[@]} -gt 0 ]] || tapes=("$here"/*.tape)

for tape in "${tapes[@]}"; do
  [[ $(basename "$tape") == common.tape ]] && continue
  echo "==> $(basename "$tape")"
  reset
  (cd "$here" && vhs "$(basename "$tape")")
done

HESTIA_HOME="$home" "$bin/hestia" daemon stop >/dev/null 2>&1 || true
echo "done — see $root/assets/demo/"
