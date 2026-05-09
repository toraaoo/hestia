BINARY_NAME := hestia
DAEMON_NAME := hestiad
BUILD_DIR   := dist
CMD_PATH    := ./cmd/hestia
DAEMON_PATH := ./cmd/hestiad

.PHONY: build test lint clean install

build:
	go build -o $(BUILD_DIR)/$(BINARY_NAME) $(CMD_PATH)
	go build -o $(BUILD_DIR)/$(DAEMON_NAME) $(DAEMON_PATH)

test:
	go test ./...

lint:
	golangci-lint run ./...

clean:
	rm -rf $(BUILD_DIR)

install:
	go install $(CMD_PATH)

release:
	goreleaser release --snapshot --clean
