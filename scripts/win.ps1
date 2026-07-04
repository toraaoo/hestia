# win.ps1 — the scripts/ dev flow on Windows (cargo).
#
#   scripts\win.ps1 build [cli|daemon|desktop|all] [-- <cargo flags>]
#   scripts\win.ps1 run   <cli|daemon> [args...]
#   scripts\win.ps1 clean
param(
  [Parameter(Position = 0)][string]$Command = "build",
  [Parameter(Position = 1)][string]$Target = "all",
  [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest
)

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

switch ($Command) {
  "build" {
    switch ($Target) {
      "cli"     { cargo build -p cli @Rest }
      "daemon"  { cargo build -p daemon @Rest }
      "desktop" { Push-Location crates\desktop; cargo tauri build @Rest; Pop-Location }
      default   { cargo build --workspace @Rest }
    }
  }
  "run" {
    switch ($Target) {
      "cli"    { cargo run -p cli -- @Rest }
      "daemon" { cargo run -p daemon -- @Rest }
      default  { Write-Error "usage: win.ps1 run <cli|daemon> [args]" }
    }
  }
  "clean" { cargo clean @Rest }
  default { Write-Error "usage: win.ps1 <build|run|clean> ..." }
}
