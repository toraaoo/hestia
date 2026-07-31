#!/usr/bin/env bash
# Compile (and optionally sign) the announcement feed from news/*.md.
#
#   scripts/announce.sh                 # compile + preview the payload
#   scripts/announce.sh new "Title"     # scaffold news/<date>-<id>.md
#   scripts/announce.sh --envelope      # wrap unsigned, for a local daemon
#   scripts/announce.sh --serve [port]  # serve the unsigned envelope on 127.0.0.1
#   scripts/announce.sh --sign          # wrap signed (CI; needs the private key)
#
# The signed envelope is what the daemon fetches: `{signature, payload}` where
# payload is the exact JSON text the signature covers. Verify, then parse — so
# the payload travels as text and is never reserialized.
#
# Local preview is --serve plus HESTIA_ANNOUNCE_ENDPOINT on the *daemon*: a
# debug build reading an overridden endpoint waives the signature check, so an
# unsigned envelope renders (see engine/src/announce/mod.rs::endpoint). Nothing
# unsigned is ever cached, and a release build has no path to the waiver.
set -euo pipefail
cd "$(dirname "$0")/.."

mode="${1:-preview}"
repo="${GITHUB_REPOSITORY:-toraaoo/hestia}"
base_url="https://github.com/${repo}/releases/download/announcements"

# Authoring writes a file rather than reading the feed, so it runs before the
# compile — a news/ that does not compile is exactly when a scaffold is useful.
if [ "$mode" = "new" ]; then
  shift
  exec python scripts/announce.py new "$@"
fi

payload="$(python scripts/announce.py compile news --base-url "$base_url")"

case "$mode" in
  preview)
    printf '%s\n' "$payload"
    ;;
  --envelope)
    jq -n --arg p "$payload" '{signature: "", payload: $p}'
    ;;
  --serve)
    port="${2:-8787}"
    # A build-dir path rather than a mktemp one, so this can `exec` below: a
    # temp dir would need an EXIT trap, and a trap cannot survive exec.
    dir="target/announce"
    rm -rf "$dir"
    mkdir -p "$dir"
    jq -n --arg p "$payload" '{signature: "", payload: $p}' > "$dir/announcements.json"
    # Images are referenced relatively in source and absolutely in the compiled
    # feed, so they have to be reachable under the same base as the document.
    if [ -d news/images ]; then cp -r news/images/. "$dir/"; fi
    cat >&2 <<EOF

serving the unsigned feed on http://127.0.0.1:$port/announcements.json

point a *debug* daemon at it, in another terminal:

  HESTIA_ANNOUNCE_ENDPOINT=http://127.0.0.1:$port/announcements.json scripts/dev.sh
  hestia daemon start && hestia news refresh

EOF
    # exec so the server *is* this pid: dev.sh/run.sh kill what they spawned,
    # and a wrapper process would leave the server orphaned behind it.
    exec python -m http.server "$port" --bind 127.0.0.1 --directory "$dir"
    ;;
  --sign)
    : "${ANNOUNCE_SIGNING_KEY:?ANNOUNCE_SIGNING_KEY is not set}"
    tmp="$(mktemp -d)"
    chmod 700 "$tmp"
    trap 'rm -rf "$tmp"' EXIT
    printf '%s' "$payload" > "$tmp/payload.json"
    # The secret is stored base64-wrapped, exactly as `tauri signer generate`
    # writes it (and as TAURI_SIGNING_PRIVATE_KEY holds its own), so setting the
    # secret is a straight copy of the file. minisign wants the raw document.
    printf '%s' "$ANNOUNCE_SIGNING_KEY" | base64 -d > "$tmp/key"
    # minisign only ever reads the password from stdin, so it is piped rather
    # than passed as an argument — an argument would be visible in the process
    # list. An unencrypted key still works: it ignores the input.
    printf '%s' "${ANNOUNCE_SIGNING_KEY_PASSWORD:-}" \
      | minisign -S -s "$tmp/key" -m "$tmp/payload.json" -x "$tmp/payload.sig" > /dev/null
    # Wrapped to match how the engine's verifier (and tauri's own signer) hands
    # a signature over: base64 around the whole minisign document.
    signature="$(base64 -w0 < "$tmp/payload.sig")"
    jq -n --arg s "$signature" --arg p "$payload" '{signature: $s, payload: $p}'
    ;;
  *)
    echo "unknown mode: $mode (preview | new | --envelope | --serve | --sign)" >&2
    exit 2
    ;;
esac
