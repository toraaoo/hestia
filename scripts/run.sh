#!/usr/bin/env bash
# Build then run a binary. Usage:
#   scripts/run.sh cli [args...]         # the hestia CLI
#   scripts/run.sh daemon [args...]      # hestiad
#   scripts/run.sh desktop               # the Tauri shell against the Vite dev server
#   scripts/run.sh --release daemon serve   # run the release build instead of debug
#   scripts/run.sh --no-news daemon serve   # skip serving news/ as the feed
#
# A debug `daemon` or `desktop` run serves news/ as the announcement feed and
# points the daemon at it. Not for `cli`, which does not fetch, and not for
# --release, which has no endpoint override to honour.
set -euo pipefail
cd "$(dirname "$0")/.."

profile=""
news=1
rest=()
for arg in "$@"; do
  case "$arg" in
    --release) profile="--release" ;;
    --no-news) news=0 ;;
    *) rest+=("$arg") ;;
  esac
done
set -- "${rest[@]}"

target="${1:-cli}"
shift || true

case "$target" in
  daemon | desktop) ;;
  *) news=0 ;;
esac
if [ -n "$profile" ]; then
  news=0
fi

if [ "$news" = 1 ]; then
  . scripts/news.sh
  # The signals matter as much as EXIT: an untrapped one skips EXIT and strands
  # the server on the port the next run wants.
  trap stop_local_feed EXIT
  trap 'stop_local_feed; exit 130' INT
  trap 'stop_local_feed; exit 143' TERM HUP PIPE
  serve_local_feed
fi

case "$target" in
  cli)     cargo run $profile -p cli -- "$@" ;;
  daemon)  cargo build $profile -p tray
           cargo run $profile -p daemon -- "$@" ;;
  desktop)
    cleanup_desktop() {
      if [ "$news" = 1 ]; then
        stop_local_feed
      fi
    }
    trap cleanup_desktop EXIT

    if [ -n "$profile" ]; then
      scripts/sidecars.sh --ensure
      (cd crates/desktop && cargo tauri build --no-bundle)
      exec ./target/release/hestia-desktop
    else
      scripts/sidecars.sh --ensure --debug
      (cd crates/desktop && cargo tauri dev)
    fi
    ;;
  *) echo "usage: $0 [--release] [cli|daemon|desktop] [args]" >&2; exit 1 ;;
esac
