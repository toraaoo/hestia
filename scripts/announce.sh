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
    echo "unknown mode: $mode (preview | --envelope | --sign)" >&2
    exit 2
    ;;
esac
