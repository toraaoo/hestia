BINARY_NAME := hestia
DAEMON_NAME := hestiad
BUILD_DIR   := dist
CMD_PATH    := ./cmd/hestia
DAEMON_PATH := ./cmd/hestiad
DATA_DIR    := $(PWD)/.hestia

.PHONY: build test lint clean install install-system install-service dev release help

build:
	go build -o $(BUILD_DIR)/$(BINARY_NAME) $(CMD_PATH)
	go build -o $(BUILD_DIR)/$(DAEMON_NAME) $(DAEMON_PATH)

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
	@HESTIA_DATA_DIR=$(DATA_DIR) PATH="$(PWD)/dist:$$PATH" $$SHELL

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
