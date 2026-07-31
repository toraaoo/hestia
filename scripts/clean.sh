#!/usr/bin/env bash
# Remove build artifacts.
. "$(dirname "$0")/lib/common.sh"
cargo clean "$@"
rm -rf frontend/dist frontend/node_modules crates/desktop/gen
