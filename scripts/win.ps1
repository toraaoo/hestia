# win.ps1 — the scripts/ dev flow on Windows (cargo).
#
#   scripts\win.ps1 build   [cli|daemon|desktop|all] [-- <cargo flags>]
#   scripts\win.ps1 run     <cli|daemon> [args...]
#   scripts\win.ps1 package [nsis|msi|portable]   # default: nsis + msi + portable zip
#   scripts\win.ps1 clean
param(
  [Parameter(Position = 0)][string]$Command = "build",
  [Parameter(Position = 1)][string]$Target = "all",
  [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest
)

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

function Stage-Sidecars {
  $triple = (rustc -vV | Select-String '^host: ').ToString().Split(' ')[1]
  cargo build --release -p cli -p daemon -p tray
  $dest = "crates\desktop\binaries"
  New-Item -ItemType Directory -Force -Path $dest | Out-Null
  foreach ($bin in "hestia", "hestiad", "tray") {
    Copy-Item "target\release\$bin.exe" "$dest\$bin-$triple.exe" -Force
  }
  return $triple
}

function New-Portable([string]$triple) {
  $version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
  $name = "hestia-$version-$triple"
  $stage = "target\package\$name"
  Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force -Path $stage | Out-Null
  foreach ($bin in "hestia", "hestiad", "tray", "hestia-desktop") {
    if (Test-Path "target\release\$bin.exe") { Copy-Item "target\release\$bin.exe" $stage -Force }
  }
  Copy-Item LICENSE, README.md $stage -Force -ErrorAction SilentlyContinue
  Compress-Archive -Path "$stage\*" -DestinationPath "target\package\$name.zip" -Force
  Write-Host "wrote target\package\$name.zip"
}

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
  "package" {
    switch ($Target) {
      "portable" { $t = Stage-Sidecars; cargo build --release -p desktop; New-Portable $t }
      "nsis"     { Stage-Sidecars | Out-Null; Push-Location crates\desktop; cargo tauri build --bundles nsis; Pop-Location }
      "msi"      { Stage-Sidecars | Out-Null; Push-Location crates\desktop; cargo tauri build --bundles msi; Pop-Location }
      default {
        $t = Stage-Sidecars
        Push-Location crates\desktop; cargo tauri build --bundles nsis,msi; Pop-Location
        New-Portable $t
      }
    }
  }
  "clean" { cargo clean @Rest }
  default { Write-Error "usage: win.ps1 <build|run|package|clean> ..." }
}
