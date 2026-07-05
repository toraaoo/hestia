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
| `win.ps1`      | the same flow on Windows (adds a `package` verb)                     |

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
```
