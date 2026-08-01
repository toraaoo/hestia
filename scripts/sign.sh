#!/usr/bin/env bash
# Sign release artifacts with the update key, writing <file>.sig beside each.
#
#   scripts/sign.sh target/release/bundle/nsis/*-setup.exe
#   scripts/sign.sh --verify target/release/bundle/deb/*.deb
#
# The signature is base64 around the whole minisign document, which is what
# engine::signature::verify_file reads and what tauri's own signer used to
# produce — so the wire form is unchanged by CI signing the artifacts itself.
#
# Needs RELEASE_SIGNING_KEY (the base64-wrapped private key, exactly as the file
# on disk) and RELEASE_SIGNING_KEY_PASSWORD.
. "$(dirname "$0")/lib/common.sh"

command -v minisign > /dev/null || die "minisign is not installed"

verify=false
if [ "${1:-}" = "--verify" ]; then
  verify=true
  shift
fi
[ $# -gt 0 ] || die "usage: $0 [--verify] <file>..."

tmp="$(mktemp -d)"
chmod 700 "$tmp"
trap 'rm -rf "$tmp"' EXIT

if [ "$verify" = true ]; then
  : "${RELEASE_PUBKEY:?RELEASE_PUBKEY is not set}"
  printf '%s' "$RELEASE_PUBKEY" | base64 -d > "$tmp/pubkey"
else
  : "${RELEASE_SIGNING_KEY:?RELEASE_SIGNING_KEY is not set}"
  printf '%s' "$RELEASE_SIGNING_KEY" | base64 -d > "$tmp/key"
fi

for file in "$@"; do
  [ -f "$file" ] || die "no such file: $file"
  if [ "$verify" = true ]; then
    base64 -d < "$file.sig" > "$tmp/sig"
    minisign -Vm "$file" -p "$tmp/pubkey" -x "$tmp/sig" > /dev/null
    log "verified $(basename "$file")"
    continue
  fi
  # The password goes over stdin, never an argument — an argument is visible in
  # the process list. An unencrypted key ignores the input.
  printf '%s' "${RELEASE_SIGNING_KEY_PASSWORD:-}" \
    | minisign -S -s "$tmp/key" -m "$file" -x "$tmp/sig" > /dev/null
  base64 -w0 < "$tmp/sig" > "$file.sig"
  log "signed $(basename "$file")"
done
