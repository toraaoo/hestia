# scripts

Thin wrappers around `cargo` and `cargo tauri` for local development and
packaging.

| Script         | What it does                                                        |
|----------------|---------------------------------------------------------------------|
| `build.sh`     | `cargo build` a target (`cli`, `daemon`, `desktop`, or `all`)        |
| `run.sh`       | build then run (`cli`, `daemon`, or `desktop` against the Vite dev server) |
| `dev.sh`       | dev subshell with `hestia`/`hestiad` on PATH; one-shot CLI; `--desktop` HMR |
| `clean.sh`     | `cargo clean` plus the frontend `dist`/`node_modules` and Tauri `gen` |
| `sidecars.sh`  | build + stage `hestia`/`hestiad`/`tray` as Tauri sidecars for bundling |
| `package.sh`   | release artifacts: Tauri installers + portable archive (`all`/`bundle`/`portable`) |
| `announce.sh`  | compile `news/*.md` into the feed: preview, serve it locally, or sign it (CI) |
| `news.sh`      | sourced by `dev.sh`/`run.sh`: serve that feed and point the daemon at it |
| `win.ps1`      | Windows entry point — forwards each verb to the matching `*.sh` via Git Bash |

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

scripts/announce.sh               # compile news/ and print the feed payload
scripts/announce.sh --serve       # serve it on 127.0.0.1:8787 by hand
scripts/dev.sh --no-news          # subshell without the local feed
```

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
