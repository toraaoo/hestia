#!/usr/bin/env bash
# Remove build artifacts.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo clean "$@"
rm -rf frontend/dist frontend/node_modules crates/desktop/gen
