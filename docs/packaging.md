# Packaging & release

How Hestia turns into installable artifacts. The packaging is driven by **CPack**
(configured in [`cmake/Packaging.cmake`](../cmake/Packaging.cmake)) plus a small
AppImage script; CI ([`.github/workflows/release.yml`](../.github/workflows/release.yml))
builds and publishes them on version tags.

## Artifacts

| Platform | Formats |
|----------|---------|
| Linux    | portable `.tar.gz`, `.deb`, `.rpm`, AppImage |
| Windows  | portable `.zip`, NSIS `.exe` |

x86_64 only for now. Builds run on Linux and Windows runners.

## Components

The install tree is split into components:

- **`daemon`** — `hestiad`. The resident core; required, every front-end needs it.
- **`cli`** — `hestia` (CLI/TUI).
- **`desktop`** — the desktop launcher, the tray helper, and the bundled CEF
  runtime.
- **`Development`** — the static libs and headers. Never packaged; build-only.

How a component maps to a package depends on the format:

- **NSIS** presents a **component picker**: `daemon` + `cli` are preselected and
  required, `desktop` is opt-in. So a default install is CLI-only.
- **`.deb` / `.rpm`** are **monolithic** — one package with all runtime
  components. (`Development` is excluded.)
- **Portable archives** bundle everything in a **flat layout** at the archive
  root (see below), built by
  [`cmake/package_portable.cmake`](../cmake/package_portable.cmake) rather than
  CPack.

The `cli` lands in `bin/`, so `hestia` is on `PATH` after install — the NSIS
installer adds the bin dir to `PATH` with the EnVar plugin (the built-in NSIS
path macro overflows when the system `PATH` is long).

## The desktop layout

CEF requires its runtime (`libcef`, `*.pak`, `locales`, blobs, sandbox) to sit
beside the executable, so the desktop installs as a self-contained unit:

- Installed packages (`.deb`/`.rpm`/installers): Linux `lib/hestia/` (with a
  `.desktop` entry + icon in `share/`); Windows a `desktop\` subdir under the
  install root, plus a Start-menu shortcut.
- Portable archives: a **flat** layout — every executable (`hestia`, `hestiad`,
  `hestia-tray`, `HestiaLauncher`) and the CEF runtime sit together at the
  archive root, so nothing is buried in `lib/`.

The on-disk binary is `HestiaLauncher` (not `Hestia`) so it doesn't collide with
the `hestia` CLI on case-insensitive Windows. The window/app identity is still
`Hestia` (`APP_NAME`/`APP_ID`); only the filename differs.

### The CEF sandbox

The sandbox helper must be SUID root. `.deb`/`.rpm` set this in a `postinst`
([`packaging/linux/postinst`](../packaging/linux/postinst)). An AppImage can't carry
a SUID binary, so the AppImage launcher runs with the sandbox disabled
(`--no-sandbox`). On Windows the sandbox uses the CEF bootstrap launcher.

## Building locally

```bash
# Everything, platform-default formats (+ AppImage if the tools are present):
scripts/package.sh

# A single generator:
scripts/package.sh TGZ
```

Packages land in `build/`. The AppImage needs `linuxdeploy` and `appimagetool`
on `PATH`.

## CI caching

Release and CI builds reuse:

- **ccache / sccache** (compiler cache) — `hendrikmuhs/ccache-action`
- **CEF** — `third_party/cef`, keyed on the CEF version
- **Bun deps** — `apps/desktop/frontend/node_modules`, keyed on `bun.lock`

First run is cold (CEF is a ~1 GB download); subsequent runs are warm.
