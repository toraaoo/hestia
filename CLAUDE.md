## Hestia

Hestia is a cross-platform Minecraft launcher written in **Rust** with a command-line interface (CLI) and a desktop
launcher (**Tauri v2**). It is designed to be fast, lightweight, and secure. It runs as a daemon (`hestiad`) with thin
clients connecting over a local socket. The project is a single cargo workspace of small path crates (one-way dependency
arrows enforced by cargo):

- `proto`: wire contracts + domain types (serde) — the no-drift `Contract` seam both sides marshal through
- `ipc`: transport (unix socket / named pipe) + JSON envelope (tokio)
- `common`: cross-cutting code — logging (`tracing`), app identity, path resolution
- `client`: the typed client SDK (one facade per domain over a `Session`)
- `engine`: the launcher engine (config, cache, download, java, accounts) — **daemon-only**; frontends reach it over IPC
- `cli` → bin `hestia` (clap + ratatui)
- `daemon` → bin `hestiad` — router, services, worker managers, process supervisor, autostart
- `tray` → bin `tray` (placeholder; not yet ported)
- `desktop` → bin `hestia-desktop` (Tauri v2 shell; UI in the root `frontend/`)

Only `daemon` depends on `engine`, so a front-end physically cannot reach launcher logic except over the socket
(`cargo tree -i engine` shows only `daemon`).

This is a greenfield project, so the codebase is small and clean. It is designed to be modular and extensible, with a
focus on maintainability and testability. The project is open-source and welcomes contributions from the community. DO
NOT CONSIDER BACKWARDS COMPATIBILITY for now, as the project is still in early development.

See @README.md and @docs/ for more information.

## Coding Guidelines

1. **Implement Logging**: Use `tracing` (configured in `common`). Log at appropriate levels (`debug`, `info`, `warn`,
   `error`). **Never log tokens or secrets.** This helps with debugging and monitoring the application.
2. **Self-Documenting Code**: Write clear, readable code. COMMENTS ARE PROHIBITED unless absolutely necessary — reserve
   them for invisible constraints (security, threading, platform quirks, wire contracts). Use descriptive names. AVOID
   RE-NARRATING THE OBVIOUS. Use `rustdoc` for public APIs and `@docs/` for design docs. Keep code and docs in sync.
3. **Defensive Programming**: Validate inputs at the edge and handle edge cases. Avoid assumptions about external data
   or state.
4. **Modular Design**: Design features in a modular way, allowing for easy testing and future extension.
5. **Single Responsibility Principle**: Each type/function should have one reason to change. Keep concerns separated.
6. **Avoid Band-Aid Solutions**: Refactor code to address root causes rather than applying quick fixes.
7. **Logging**: Use structured logging with appropriate log levels. Avoid logging sensitive information.
8. **YAGNI (You Aren't Gonna Need It)**: Only implement what's needed now. Avoid speculative features.
9. **DRY (Don't Repeat Yourself)**: Extract repeated logic into reusable functions or types.
10. **Simplicity Over Complexity**: Prefer straightforward solutions. Avoid over-engineering.
11. **Explicit Over Implicit**: Be clear about dependencies and behavior. Avoid magic or hidden side effects.
12. **Immutable Data Structures**: Favor immutability where possible, especially in domain entities and DTOs.
13. **Consistent Naming Conventions**: Follow Rust conventions; single-word module names (`config`, not `config_store`).
14. **Error Handling**: Use `thiserror` enums in libraries, mapped to `ipc::ErrorCode` at the service boundary (the
    daemon's `ServiceError`); `anyhow` at binary edges. Don't panic on recoverable errors.
15. **Security Best Practices**: Validate and sanitize input. Use secure defaults and never hardcode secrets.
16. **Dependency Management**: Minimize external dependencies. Prefer native implementations or well-maintained,
    widely-used crates. Pin shared versions in `[workspace.dependencies]`.
17. **One concept per file**: Split a file when it holds two concepts a caller can name separately — as `backup.rs` did,
    with the schedule vocabulary beside the archive lifecycle. Length is a hint, never the test: `proto/src/error.rs` is
    800 lines of one enum and two matches over it, and splitting it would only scatter the vocabulary. Apply the
    deletion test instead — if the module vanished, would complexity concentrate in one place or spread across its
    callers?

You should always keep in mind that this project is a cross-platform application. Avoid platform-specific code unless
absolutely necessary. If platform-specific code is required, isolate it behind `#[cfg(...)]` with a shared
trait/interface so callers stay platform-agnostic. Always test your code on all supported platforms to ensure
compatibility and functionality.

Based on the rules above, follow these coding guidelines:
YAGNI and DRY principles should be applied throughout the codebase. Code should be simple, self-documenting, and follow
the single responsibility principle. Use explicit naming conventions and immutable data structures where appropriate.
Handle errors with `Result`/`thiserror` and log information at appropriate levels. Follow security best practices to
ensure the application is secure. `rustfmt` + `clippy -D warnings` must stay clean.

If asked to implement a new feature, follow these steps:

1. Define the feature requirements and scope.
2. Design the feature with simplicity and maintainability in mind.
3. Implement the feature in a modular and reusable way.
    - Ask these questions before implementing a new feature:
        - Is this feature necessary for the current requirements?
        - Can this feature be implemented in a simpler way? Consider the following options in hierarchy:
            - natively - can it be implemented without any external dependencies?
            - existing crates - can it be implemented using a crate already in the workspace?
            - add a new dependency - if a new dependency is necessary, ensure it is well-maintained and widely used.
        - Are there any existing components that can be reused?

The wire-in points are one-liners (see @docs/contributing.md): a `Contract` in `proto`, a `handle::<C>` in the daemon's
`services.rs`, a facade method in `client`, a `clap` subcommand in `cli`, a `#[tauri::command]` in `desktop`.

See @docs/architecture.md and @docs/contributing.md for more information on the project architecture and design
principles.

## Windows Box

There's a Windows box available for testing and debugging. It is accessible via ssh at `ssh win` on the local network.
The box is running Windows 11 and has the necessary tools installed for building and testing the project. Use it to test
platform-specific code and ensure compatibility with Windows. Project is located at `~/Projects/hestia` on the Windows
box. You may commit and push changes from the Windows box see the next sections for guidelines. Use the Windows box for
testing and debugging only.

## Commit Guidelines

Commit things as you go, but avoid committing work-in-progress changes. Each commit should represent a single logical
change and have a clear, concise commit message. Follow these guidelines for committing changes:

1. **Atomic Commits**: Each commit should represent a single logical change. Avoid mixing unrelated changes in one
   commit.
2. **Descriptive Commit Messages**: Use clear, concise commit single line commit message with ~50 characters. Following
   the format: `<type>(<scope>): <description>` (e.g., `feat(daemon): add process supervisor`).
3. **Reference Issues/Tasks**: If the commit relates to an issue or task, reference it in the commit message (e.g.,
   `Fixes #123`).
4. **Avoid WIP Commits**: Avoid committing work-in-progress changes. Use feature branches for ongoing work.
5. Do not add co-authors or other metadata to commits. The commit history should reflect the actual changes made, not
   the contributors.
6. Do not include sensitive information in commit messages. Avoid including passwords, API keys, or other secrets.
7. Leave CLAUDE.md untracked in commits. It is a local configuration file and should not be part of the version control
   history.
8. Run `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` before committing to ensure code
   formatting and linting are clean.
