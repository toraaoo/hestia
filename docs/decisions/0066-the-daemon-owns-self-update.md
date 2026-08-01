# The daemon owns self-update; the shell asks like everything else

*Applies to: [The engine](../architecture/engine.md), [Front-ends](../architecture/frontends.md), [Packaging](../packaging.md)*

Self-update was two implementations of one feature. The daemon read
`latest.json`, verified it against `update_pubkeys()` and downloaded an
installer, exposed over `update.*` for the CLI. The desktop shell read the *same*
`latest.json` through `tauri-plugin-updater`, from an endpoint and a key written
a second time in `tauri.conf.json`, verified it against that copy, and installed
it. Two HTTP clients, two manifest parsers, two verifiers, two error models, and
a test whose only job was to stop the two copies of the endpoint drifting apart.

Both sides were also wrong in ways the other was not. The plugin holds exactly
one `pubkey`, so the shell faked key rotation by building an updater per key and
**re-downloading** for each. The daemon's side could not apply anything on Linux
at all, and its Windows apply used `Command::spawn` — which is `CreateProcess`,
which refuses a `highestAvailable` manifest with `ERROR_ELEVATION_REQUIRED`
instead of raising the UAC prompt the plugin got by using `ShellExecuteW`.

**The daemon owns it.** `engine/src/update/` does the check, the download, the
verification and the apply; `update.check`, `update.download` and `update.apply`
are the only way in, and `tauri-plugin-updater` is gone.

What decided it was not tidiness. Two requirements were fixed: a CLI-or-daemon
install with no desktop app must be able to update itself, and deb, rpm and
AppImage must all update in place. `hestiad` has no Tauri runtime, so the plugin
cannot serve the headless case — meaning the engine had to grow the full Linux
apply *regardless*. Once it has one, the plugin does nothing the engine does not.

The middle option — engine checks and downloads, plugin applies — was measured
and rejected: the plugin's `Update` has private fields, so it is obtainable only
from its own `check()`, which re-fetches, re-parses and re-verifies with the one
key it holds. It preserves every duplicate while still leaving the Linux apply to
write.

## What this costs

About 150 lines that upstream used to maintain: the AppImage rename, `dpkg -i` /
`rpm -U`, and the escalation ladder. That is the price of the two requirements,
not of this decision — and `install.rs` had to exist either way, because which
artifact to *download* depends on how the copy was installed.

## What follows from it

- The endpoint and the trusted keys are written once, in `common::app`. The
  drift guard now asserts the duplicate has **not** come back.
- Key rotation works everywhere, verifying a key set rather than one key, with
  no re-download per candidate.
- `latest.json` grows an additive `formats` map, since one platform key can hold
  one URL and Linux now ships three artifacts.
- CI signs artifacts itself with `minisign` — the bundler never signed `.deb` or
  `.rpm`, which is exactly what `formats` needs — so `TAURI_SIGNING_*`
  disappears in favour of `RELEASE_SIGNING_KEY`, matching `ANNOUNCE_SIGNING_KEY`.
- Elevation is *asked for*, not refused for want of it: `pkexec` unconditionally
  (a text polkit agent would have answered, so gating on `$DISPLAY` declines on
  its behalf), then passwordless `sudo`. Only when nothing can even ask does the
  daemon return `ElevationRequired` with the exact command — and `hestia update`
  offers to run it, because the CLI has the terminal the daemon lacks.
- A portable build compiles detection out entirely. An archive unpacked by hand
  has no installer to update through, and a path heuristic could be defeated by
  dropping one under `/usr/bin`.

The shell keeps exactly one bespoke command, `changelog`, and for the same
reason it always had: the notes are compiled into the binary and shown on the
first run *after* an update, which is when the network is least trustworthy.
