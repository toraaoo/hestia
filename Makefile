BINARY_NAME := hestia
DAEMON_NAME := hestiad
BUILD_DIR   := dist
CMD_PATH    := ./cmd/hestia
DAEMON_PATH := ./cmd/hestiad
DATA_DIR    := $(PWD)/.hestia

VERSION_PKG := github.com/toraaoo/hestia/internal/version

# Prefer git tags for versioning.
# - If HEAD is exactly at a tag: use that tag (eg. v0.3.0)
# - Otherwise: use latest tag + "-dev" (eg. v0.3.0-dev)
GIT_TAG_LATEST := $(shell git describe --tags --abbrev=0 2>/dev/null || echo dev)
GIT_TAG_EXACT  := $(shell git describe --tags --exact-match 2>/dev/null)
ifeq ($(strip $(GIT_TAG_EXACT)),)
  ifeq ($(strip $(GIT_TAG_LATEST)),dev)
    VERSION := dev
  else
    VERSION := $(GIT_TAG_LATEST)-dev
  endif
else
  VERSION := $(GIT_TAG_EXACT)
endif

GIT_COMMIT := $(shell git rev-parse --short=12 HEAD 2>/dev/null || echo unknown)
BUILD_DATE := $(shell date -u +%Y-%m-%dT%H:%M:%SZ)

LDFLAGS := -s -w \
	-X $(VERSION_PKG).Version=$(VERSION) \
	-X $(VERSION_PKG).GitCommit=$(GIT_COMMIT) \
	-X $(VERSION_PKG).BuildDate=$(BUILD_DATE)

MAKEFLAGS += --no-print-directory

.PHONY: build test lint clean install install-system install-service dev release help

build:
ifeq ($(DEV),1)
	@hestia daemon stop > /dev/null 2>&1 || true
endif
	go build -ldflags "$(LDFLAGS)" -o $(BUILD_DIR)/$(BINARY_NAME) $(CMD_PATH)
	go build -ldflags "$(LDFLAGS)" -o $(BUILD_DIR)/$(DAEMON_NAME) $(DAEMON_PATH)

test:
	go test ./...

lint:
	golangci-lint run ./...

clean:
	rm -rf $(BUILD_DIR) .hestia

install:
	go install $(CMD_PATH)
	go install $(DAEMON_PATH)

install-system: build
	install -Dm755 $(BUILD_DIR)/$(BINARY_NAME) /usr/local/bin/$(BINARY_NAME)
	install -Dm755 $(BUILD_DIR)/$(DAEMON_NAME) /usr/local/bin/$(DAEMON_NAME)

install-service: install-system
	install -Dm644 configs/hestiad.service /etc/systemd/system/hestiad.service
	systemctl daemon-reload
	@echo "Enable with: systemctl enable --now hestiad"

dev: build
ifeq ($(DEV),1)
	@echo "Already in a dev shell! Exit first or just keep working."
else
	@HESTIA_DATA_DIR=$(DATA_DIR) \
	PATH="$(PWD)/dist:$$PATH" \
	DEV=1 \
	exec $$SHELL
endif

release:
	goreleaser release --snapshot --clean

help:
	@echo "make build          - Build both binaries"
	@echo "make dev            - Build + spawn shell with env set"
	@echo "make install        - Install both binaries via go install"
	@echo "make install-system - Install to /usr/local/bin"
	@echo "make install-service - Install + systemd service"
	@echo "make test           - Run tests"
	@echo "make lint           - Run linter"
	@echo "make clean          - Remove artifacts"
	@echo "make release        - Snapshot release"
