#!/usr/bin/env bash
# Compile (and optionally sign) the announcement feed from news/*.md.
#
#   scripts/announce.sh                 # compile + preview the payload
#   scripts/announce.sh --envelope      # wrap unsigned, for a local daemon
#   scripts/announce.sh --sign          # wrap signed (CI; needs the private key)
#
# The signed envelope is what the daemon fetches: `{signature, payload}` where
# payload is the exact JSON text the signature covers. Verify, then parse — so
# the payload travels as text and is never reserialized.
#
# Local preview: point a debug build at the unsigned output with
#   HESTIA_ANNOUNCE_ENDPOINT=http://127.0.0.1:8000/announcements.json
# It will be *refused* (no key verifies it) — which is the point of the check;
# use --sign with a throwaway key to exercise the rendering path.
set -euo pipefail
cd "$(dirname "$0")/.."

mode="${1:-preview}"
repo="${GITHUB_REPOSITORY:-toraaoo/hestia}"
base_url="https://github.com/${repo}/releases/download/announcements"

payload="$(python3 scripts/announce.py news "$base_url")"

case "$mode" in
  preview)
    printf '%s\n' "$payload"
    ;;
  --envelope)
    jq -n --arg p "$payload" '{signature: "", payload: $p}'
    ;;
  --sign)
    : "${ANNOUNCE_SIGNING_KEY:?ANNOUNCE_SIGNING_KEY is not set}"
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    printf '%s' "$payload" > "$tmp/payload.json"
    printf '%s' "$ANNOUNCE_SIGNING_KEY" > "$tmp/key"
    # minisign writes a two-line document; the wire carries it base64-wrapped,
    # matching how tauri's signer hands over an artifact signature.
    minisign -S -s "$tmp/key" -m "$tmp/payload.json" -x "$tmp/payload.sig" \
      ${ANNOUNCE_SIGNING_KEY_PASSWORD:+-W} > /dev/null
    signature="$(base64 -w0 < "$tmp/payload.sig")"
    jq -n --arg s "$signature" --arg p "$payload" '{signature: $s, payload: $p}'
    ;;
  *)
    echo "unknown mode: $mode (preview | --envelope | --sign)" >&2
    exit 2
    ;;
esac
