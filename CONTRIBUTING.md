# Contributing to Hestia

Thanks for your interest in contributing to Hestia!

## Getting Started

### Prerequisites

- Go 1.26+
- Make

### Building

```sh
git clone https://github.com/toraaoo/hestia
cd hestia
make build
```

Binaries output to `dist/hestia` and `dist/hestiad`.

### Running Tests

```sh
make test
```

### Linting

```sh
make lint
```

Requires [golangci-lint](https://golangci-lint.run/). Install with:

```sh
go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest
```

## Project Structure

```
hestia/
├── cmd/
│   ├── hestia/       # CLI entry point
│   └── hestiad/      # Daemon entry point
├── internal/
│   ├── cli/          # CLI commands (cobra)
│   ├── daemon/       # Daemon + API handlers
│   ├── client/       # HTTP client for CLI→daemon
│   ├── server/       # Server config and state
│   ├── jar/          # JAR providers (vanilla, paper, fabric)
│   ├── jre/          # JRE management
│   ├── backup/       # Backup system
│   └── config/       # Global config
└── docs/             # Documentation
```

See [docs/architecture.md](docs/architecture.md) for detailed architecture.

## Development Guidelines

### Code Style

- Follow standard Go conventions
- Use `gofmt` for formatting
- Keep functions small and focused
- Avoid global state

### Dependency Rule

```
cli/commands/* → client → socket → daemon → process
```

No layer skips another. CLI never imports `daemon` or `process` directly.

### Adding Commands

1. Create file in `internal/cli/commands/<group>/`
2. Implement `new<Name>Cmd() *cobra.Command`
3. Add to parent command in `<group>.go`

### Adding JAR Providers

1. Create file in `internal/jar/providers/`
2. Implement `jar.Provider` interface
3. Register in `internal/jar/registry.go`

### Adding API Endpoints

1. Add handler in `internal/daemon/api/`
2. Register route in `internal/daemon/api/api.go`
3. Add client method in `internal/client/client.go`

## Pull Requests

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run tests and linter
5. Commit with clear message
6. Push and open PR

### Commit Messages

Use conventional commits:

```
feat: add fabric jar provider
fix: prevent crash on empty server list
docs: update installation instructions
refactor: simplify backup retention logic
test: add coverage for JRE downloader
```

### PR Checklist

- [ ] Tests pass (`make test`)
- [ ] Linter passes (`make lint`)
- [ ] New code has tests where appropriate
- [ ] Documentation updated if needed

## Reporting Issues

When reporting bugs, include:

- Hestia version (`hestia --version`)
- OS and architecture
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
