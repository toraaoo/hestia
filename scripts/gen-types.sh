#!/usr/bin/env bash
# Regenerate the TypeScript bindings for the wire error surface from the Rust
# `proto` types (ts-rs). The generated files land in
# `frontend/src/api/types/generated/` and are committed — run this whenever a
# `proto::error::ErrorInfo` variant (or a token enum it references) changes.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "generating TypeScript error bindings from proto…"
cargo test -p proto --features ts >/dev/null
echo "wrote frontend/src/api/types/generated/"
