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
on install. `deb`/`rpm`/AppImage and the MSI install the **full set** with no
component picker; the NSIS installer is customized — see below.

## The NSIS installer

Windows uses a **custom NSIS template**
([`crates/desktop/windows/installer.nsi`](../crates/desktop/windows/installer.nsi)),
a fork of tauri-bundler's stock template rendered with the same handlebars
context. It must track the pinned tauri-cli version (`2.10.1`, both locally and
in [`release.yml`](../.github/workflows/release.yml)) — re-diff the fork against
upstream's `crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi` when
bumping.

What it adds over stock:

- **Components page** — *Hestia core* (`hestiad` + `tray`, required),
  *Desktop app*, and *CLI* (both checked by default, deselectable). Choices are
  persisted in the uninstall registry key and become the defaults for the next
  run — and the effective selection for silent/passive updates. Deselecting a
  previously installed component on an update removes it.
- **Install mode `both`** — per-user or all-users, chosen at install time
  (`bundle.windows.nsis.installMode` in `tauri.conf.json`), remembered for
  updates and uninstall.
- **CLI on PATH** — the CLI component appends the install directory to the
  user or machine `PATH` (matching the install mode) and removes it on
  uninstall or deselection.
- **Graceful daemon handling** — before files are swapped, a running `hestiad`
  is asked to stop via `hestiad stop` (supervised game servers keep running —
  the daemon re-adopts them on its next start) and only killed if it lingers;
  the tray is stopped too. If the daemon was running, the installer restarts
  it afterwards (unelevated, hidden window).
- **Data survives uninstall** — `%APPDATA%\Hestia` (instances, servers,
  worlds, accounts) is only removed when the uninstaller's *delete app data*
  box is explicitly ticked. Uninstall also drops the `Hestia Daemon`
  autostart scheduled task.
- **Update-friendly** — running a newer setup over an existing install
  upgrades in place (no uninstall detour), reusing the recorded install
  directory, install mode, and components; downgrades take the stock
  uninstall-first path.

## Auto-updates

The desktop app ships `tauri-plugin-updater`, polling
`https://github.com/toraaoo/hestia/releases/latest/download/latest.json`. On a
`v*` tag, the release workflow's `manifest` job composes `latest.json` from the
signed NSIS setup (`windows-x86_64`) and AppImage (`linux-x86_64`) and attaches
it to the Release. On Windows the update runs the NSIS installer passively, so
the custom template's remembered install dir + components apply.

Updater artifacts are signed (`bundle.createUpdaterArtifacts`); the public key
lives in `tauri.conf.json`, and CI signs with two repository secrets:

- `TAURI_SIGNING_PRIVATE_KEY` — the private key (generated with
  `cargo tauri signer generate`; **losing it means shipped apps can no longer
  accept updates**).
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — its password (empty if the key has
  none).

Local `scripts/package.sh` runs without the key and simply skips the
signatures/updater artifacts.

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
