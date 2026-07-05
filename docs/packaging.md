# Packaging & release

How Hestia turns into installable artifacts. Packaging is driven by the **Tauri
v2 bundler** (`cargo tauri build`); the desktop app and its sidecar binaries are
bundled together, and CI ([`.github/workflows/release.yml`](../.github/workflows/release.yml))
builds and publishes the artifacts on version tags.

## Artifacts

| Platform | Formats                                          |
|----------|--------------------------------------------------|
| Linux    | `.deb`, `.rpm`, AppImage, portable `.tar.gz`     |
| Windows  | NSIS `.exe`, WiX `.msi`, portable `.zip`         |

x86_64 only for now. Builds run on Linux and Windows runners (the MSI can only be
produced on Windows).

## One installer, everything bundled

The desktop app is the product; the daemon, tray, and CLI ride along as Tauri
**sidecars** (`bundle.externalBin` in
[`crates/desktop/tauri.conf.json`](../crates/desktop/tauri.conf.json)):

- `hestiad` — the resident daemon the desktop app drives over the socket.
- `tray` — the system-tray helper.
- `hestia` — the CLI/TUI.

Each is built with the target-triple suffix Tauri requires
(`hestiad-x86_64-unknown-linux-gnu`, …) and staged into `crates/desktop/binaries/`
by [`scripts/sidecars.sh`](../scripts/sidecars.sh); the bundler strips the suffix
on install. Every installer and every bundle installs the **full set** — there is
no component picker. `deb`/`rpm` are monolithic; NSIS and MSI use the stock Tauri
installers.

The **portable archives** are the same four binaries (`hestia`, `hestiad`,
`tray`, `hestia-desktop`) plus `LICENSE`/`README`, packed flat by
[`scripts/package.sh`](../scripts/package.sh) — Tauri has no portable target, so
this is a plain `tar`/`Compress-Archive` step.

## Runtime dependency: the system WebView

The desktop binary needs a system WebView — **WebView2** on Windows (present by
default on Windows 10/11) and **WebKitGTK** on Linux. The `.deb` declares the
WebKitGTK/GTK/appindicator packages under `bundle.linux.deb.depends`; the AppImage
carries what it can. The portable archives assume the WebView is already present.

## Building locally

```bash
# Sidecars + platform installers + portable archive:
scripts/package.sh

# Just the installers, or just the portable archive:
scripts/package.sh bundle
scripts/package.sh portable
```

`cargo tauri build` runs the frontend build itself (via `beforeBuildCommand`), so
only `bun install` (in `frontend/`) and a staged sidecar set are prerequisites.
Bundles land in `target/release/bundle/{deb,rpm,appimage,nsis,msi}/`; portable
archives in `target/package/`. On Windows use `scripts\win.ps1 package`; the MSI
additionally needs the WiX Toolset the bundler fetches on first run.

## CI

- [`ci.yml`](../.github/workflows/ci.yml) — `fmt` + `clippy` + `test` on Linux and
  Windows, plus `cargo-deny`. The Linux job installs the WebKitGTK dev packages so
  the `desktop` crate compiles.
- [`release.yml`](../.github/workflows/release.yml) — on a `v*` tag, runs
  `scripts/package.sh all` on a Linux and a Windows runner and attaches every
  artifact to the GitHub Release. A manual `workflow_dispatch` is a dry run: it
  builds and uploads workflow artifacts but does not touch a Release.
