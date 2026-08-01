#!/usr/bin/env bash
# Serve a fake release feed so the update path can be driven locally.
#
#   eval "$(scripts/update.sh --env)"  # point this shell at the local feed
#   scripts/update.sh --serve [port]   # serve it on 127.0.0.1
#   scripts/update.sh                  # compile a latest.json and print it
#
# The artifact is a throwaway file, but it is *signed*, and the signature is
# still checked — verification is the part of the path most worth exercising.
#
# Nothing here edits the source. The key is passed in HESTIA_UPDATE_PUBKEY,
# which a **debug** build honours only alongside HESTIA_UPDATE_ENDPOINT, so a
# shipped binary cannot be pointed anywhere and there is nothing to put back.
# The keypair is generated on first use into target/update/, which `cargo clean`
# takes with it.
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

ensure_keys() {
  [ -f "$key" ] && return 0
  mkdir -p "$dir"
  minisign -G -f -s "$key" -p "$key.pub" -W > /dev/null
  log "generated $key" >&2
}

compile() {
  ensure_keys
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
  --env)
    ensure_keys
    echo "export HESTIA_UPDATE_ENDPOINT=http://127.0.0.1:$port/latest.json"
    echo "export HESTIA_UPDATE_PUBKEY=$(base64 -w0 < "$key.pub")"
    ;;
  --serve)
    compile > /dev/null
    log "update feed on http://127.0.0.1:$port/latest.json (version $version)"
    log "point a shell at it with:  eval \"\$($0 --env)\""
    # exec so the server *is* this pid, matching announce.sh.
    exec python -m http.server "$port" --bind 127.0.0.1 --directory "$dir"
    ;;
  "") compile ;;
  *) die "unknown mode: $1 (--env | --serve | <none>)" ;;
esac
