# scripts

Thin wrappers around `cargo`, `cargo tauri`, and `cargo dist` for local
development and CI.

| Script         | What it does                                                        |
|----------------|---------------------------------------------------------------------|
| `build.sh`     | `cargo build` a target (`cli`, `daemon`, `desktop`, or `all`)        |
| `run.sh`       | build then run (`cli`, `daemon`, or `desktop` against the Vite dev server) |
| `dev.sh`       | dev subshell with `hestia`/`hestiad` on PATH; one-shot CLI; `--desktop` HMR |
| `clean.sh`     | `cargo clean` plus the frontend `dist`/`node_modules` and Tauri `gen` |
| `package.sh`   | release artifacts: `cargo dist build` (cli/daemon) or the Tauri bundle |
| `win.ps1`      | the same flow on Windows                                             |

Examples:

```bash
scripts/build.sh cli --release
scripts/run.sh daemon serve
scripts/run.sh desktop            # Tauri shell + Vite HMR
scripts/package.sh cli

scripts/dev.sh                    # subshell: hestia + hestiad on PATH
scripts/dev.sh java list          # one-shot CLI (builds first)
scripts/dev.sh --desktop          # desktop shell with frontend HMR
```
