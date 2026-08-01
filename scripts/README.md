# scripts

Thin wrappers around `cargo` and `cargo tauri` for local development and
packaging.

```
scripts/
├── <verb>.sh        one file per verb — what you run
├── win.ps1          the Windows entry point
└── lib/             what a verb uses; never run directly
```

**A file at the top level is a verb.** `win.ps1` discovers `scripts/*.sh` and
forwards to whichever one matches, so a new script is reachable from PowerShell
the moment it exists — there is no list to update. Anything a verb *sources* or
*drives* is not a verb and lives in `lib/`, which the discovery glob does not
reach: that directory is the whole distinction, and it is why the split is worth
having.

| Verb            | What it does                                                        |
|-----------------|---------------------------------------------------------------------|
| `build.sh`      | `cargo build` a target (`cli`, `daemon`, `desktop`, or `all`)        |
| `run.sh`        | build then run (`cli`, `daemon`, or `desktop` against the Vite dev server) |
| `dev.sh`        | dev subshell with `hestia`/`hestiad` on PATH; one-shot CLI; `--desktop` HMR |
| `clean.sh`      | `cargo clean` plus the frontend `dist`/`node_modules` and Tauri `gen` |
| `sidecars.sh`   | build + stage `hestia`/`hestiad`/`tray` as Tauri sidecars for bundling |
| `package.sh`    | release artifacts: Tauri installers + portable archive (`all`/`bundle`/`portable`) |
| `announce.sh`   | announcements: scaffold one (`new`), compile `news/*.md` into the feed, serve it locally, or sign it (CI) |
| `update.sh`     | self-update: serve a signed fake `latest.json` locally, and print the env to reach it |
| `sign.sh`       | minisign release artifacts (`<file>.sig`), or verify them (CI)      |
| `gen-types.sh`  | regenerate the TypeScript bindings for the `proto` wire types (ts-rs) |
| `gen-icons.sh`  | regenerate every shipped icon from `assets/icons/ember.svg`           |

| Helper             | Used by                                                          |
|--------------------|------------------------------------------------------------------|
| `lib/common.sh`    | sourced first by every verb: strict mode, the repo root as cwd, `log`/`die` |
| `lib/news.sh`      | sourced by `dev.sh`/`run.sh`: serve the feed and point the daemon at it |
| `lib/announce.py`  | `announce.sh` — compiles `news/*.md`, and scaffolds a new entry     |
| `lib/gen-barrels.py` | `gen-types.sh` — the per-module barrels over the generated types |

A verb starts with one line, and everything after it can assume the repo root:

```bash
#!/usr/bin/env bash
. "$(dirname "$0")/lib/common.sh"
```

Examples:

```bash
scripts/build.sh cli --release
scripts/run.sh daemon serve
scripts/run.sh desktop            # Tauri shell + Vite HMR
scripts/package.sh                # sidecars + Tauri bundles + portable archive
scripts/package.sh portable       # portable .tar.gz only

scripts/dev.sh                    # subshell: hestia + hestiad on PATH
scripts/dev.sh java list          # one-shot CLI (builds first)
scripts/dev.sh --desktop          # desktop shell with frontend HMR

scripts/announce.sh new "Title"   # scaffold news/<date>-<id>.md, then edit it
scripts/announce.sh               # compile news/ and print the feed payload
scripts/announce.sh --serve       # serve it on 127.0.0.1:8787 by hand
scripts/dev.sh --no-news          # subshell without the local feed

eval "$(scripts/update.sh --env)"  # point this shell at the local feed
scripts/update.sh --serve         # fake release feed on 127.0.0.1:8788
```

**Testing self-update locally.** Unlike the news feed this is opt-in, because a
dev run should not offer a fake update every time. Nothing here edits the source:
`--env` prints the endpoint *and* the key to trust, and a debug build honours
`HESTIA_UPDATE_PUBKEY` only alongside `HESTIA_UPDATE_ENDPOINT` — so there is no
constant to paste in and nothing to remember to put back. A release build reads
neither.

```bash
scripts/update.sh --serve &
eval "$(scripts/update.sh --env)"
scripts/dev.sh
hestia update --yes
```

The served artifact is a throwaway file but it is **signed**, and the signature
is still checked — that is the part of the path most worth exercising. Drop
`HESTIA_UPDATE_PUBKEY` and the same run fails with *no trusted key verifies this
artifact*, which is the compiled-in key set doing its job.

A `target/` build is *unmanaged*, so that run stops at "download it at …" — the
check and the version comparison, and no more. To reach the download, the
verification and the apply, tell the daemon it is an AppImage, the one install
shape that needs no root:

```bash
printf 'old\n' > /tmp/Hestia.AppImage && chmod +x /tmp/Hestia.AppImage
APPIMAGE=/tmp/Hestia.AppImage scripts/dev.sh
hestia update --yes
cat /tmp/Hestia.AppImage      # replaced, and still 0755
```

`deb`/`rpm` cannot be faked this way — detection asks `dpkg-query -S` / `rpm -qf`
who owns the binary, so it needs a real package install. Windows needs a real
NSIS install for the same reason: the uninstaller at the layout root is what
`update/install.rs` looks for.

A debug `dev.sh`, `run.sh daemon` or `run.sh desktop` **serves `news/` as the
announcement feed** and points the daemon it starts at it, so an entry can be
seen before it is published. `--no-news` skips it, `HESTIA_NEWS_PORT` moves it
off 8787, and a `--release` run never does it — only a debug binary honours the
endpoint override. Nothing here fails a run: if `news/` does not compile, or the
port is taken by something else, the feed is skipped with a note and the daemon
falls back to the published one. The exception is bare `win.ps1 dev`, which is
pure PowerShell and needs only cargo; a forwarded verb (`win.ps1 run daemon
serve`) serves the feed as usual.

See [news/README.md](../news/README.md) for the announcement format and the
publishing path.
