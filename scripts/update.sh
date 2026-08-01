#!/usr/bin/env bash
# Serve a fake release feed so the update path can be driven locally.
#
#   scripts/update.sh                 # compile a latest.json and print it
#   scripts/update.sh --serve [port]  # serve it on 127.0.0.1
#   scripts/update.sh --keys          # generate the local dev keypair
#
# The artifact is a throwaway file, but it is *signed*, with a key this build
# trusts — verification is the part of the path most worth exercising, so it is
# not waived. `--keys` writes target/update/dev.key and prints the public half
# to paste into common::app::UPDATE_PUBKEY for the duration of a test.
#
# Only a debug build reads HESTIA_UPDATE_ENDPOINT, so nothing here can point a
# shipped binary anywhere.
. "$(dirname "$0")/lib/common.sh"

command -v minisign > /dev/null || die "minisign is not installed"

dir="target/update"
key="$dir/dev.key"
version="${HESTIA_UPDATE_VERSION:-99.0.0}"
port="${2:-8788}"

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) target="windows-x86_64" ;;
  Darwin) target="macos-x86_64" ;;
  *) target="linux-x86_64" ;;
esac

generate_keys() {
  mkdir -p "$dir"
  rm -f "$key" "$key.pub"
  minisign -G -f -s "$key" -p "$key.pub" -W > /dev/null
  log "wrote $key"
  echo
  echo "paste this into crates/common/src/app.rs as UPDATE_PUBKEY_NEXT:"
  base64 -w0 < "$key.pub"
  echo
}

compile() {
  [ -f "$key" ] || die "no dev key — run: $0 --keys"
  mkdir -p "$dir"

  # Stands in for an installer. Applying it will fail, which is the point at
  # which a real artifact is swapped in; everything before that is exercised.
  local artifact="$dir/hestia-$version-fake"
  printf 'not a real installer — %s\n' "$version" > "$artifact"
  minisign -S -s "$key" -m "$artifact" -x "$dir/artifact.sig" > /dev/null 2>&1

  local url="http://127.0.0.1:$port/$(basename "$artifact")"
  local sig
  sig="$(base64 -w0 < "$dir/artifact.sig")"
  jq -n --arg v "$version" --arg t "$target" --arg url "$url" --arg sig "$sig" \
    '{version: $v, notes: "A local test release.", platforms: {($t): {url: $url, signature: $sig}}}' \
    > "$dir/latest.json"
  cat "$dir/latest.json"
}

case "${1:-}" in
  --keys) generate_keys ;;
  --serve)
    compile > /dev/null
    log "update feed on http://127.0.0.1:$port/latest.json (version $version)"
    log "point a debug daemon at it:"
    log "  HESTIA_UPDATE_ENDPOINT=http://127.0.0.1:$port/latest.json scripts/dev.sh"
    # exec so the server *is* this pid, matching announce.sh.
    exec python -m http.server "$port" --bind 127.0.0.1 --directory "$dir"
    ;;
  "") compile ;;
  *) die "unknown mode: $1 (--keys | --serve | <none>)" ;;
esac
