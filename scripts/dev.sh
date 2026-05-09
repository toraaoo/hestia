#!/usr/bin/env bash
set -e

export HESTIA_DATA_DIR="${HESTIA_DATA_DIR:=$(pwd)/.hestia}"
export PATH="$PATH:$(pwd)/dist"
echo "Using data dir: $HESTIA_DATA_DIR"

go build -o dist/hestia ./cmd/hestia
go build -o dist/hestiad ./cmd/hestiad

exec ./dist/hestia "$@"
