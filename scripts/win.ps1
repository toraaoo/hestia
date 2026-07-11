# win.ps1 — run the scripts/ dev flow on Windows from PowerShell.
#
# Build/run/package verbs forward to the matching scripts/*.sh, so the shell
# scripts stay the single source of truth (no duplicated packaging logic). Two
# things PowerShell owns natively rather than delegating to Git Bash: stopping
# every running hestiad before a build (a live hestiad.exe hard-locks the .exe
# cargo is about to overwrite), and the interactive `dev` subshell — bare
# `win.ps1 dev` drops you into PowerShell, not `bash -i`.
#
# Requires Git for Windows (bash.exe) for the forwarded verbs. WSL's bash is
# intentionally skipped: it runs a Linux userland that can't see the Windows
# cargo target. Bare `dev` is pure PowerShell and needs only cargo.
#
#   scripts\win.ps1 dev      [--desktop] [hestia args...]   # PS subshell / one-shot CLI
#   scripts\win.ps1 build    [cli|daemon|desktop|all] [cargo flags...]
#   scripts\win.ps1 run      <cli|daemon|desktop> [args...]
#   scripts\win.ps1 sidecars [target-triple]
#   scripts\win.ps1 package  [all|bundle|portable]
#   scripts\win.ps1 clean    [cargo flags...]
param(
  [Parameter(Position = 0)][string]$Command,
  [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest
)

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

function Find-Bash {
  $candidates = @(
    "$env:ProgramFiles\Git\bin\bash.exe",
    "${env:ProgramFiles(x86)}\Git\bin\bash.exe",
    "$env:LOCALAPPDATA\Programs\Git\bin\bash.exe"
  )
  foreach ($c in $candidates) { if ($c -and (Test-Path $c)) { return $c } }
  # PATH fallback, but never System32\bash.exe (the WSL launcher).
  $cmd = Get-Command bash.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.Source -notmatch 'System32' } |
    Select-Object -First 1
  if ($cmd) { return $cmd.Source }
  throw "Git Bash not found. Install Git for Windows: https://git-scm.com/download/win"
}

# Stop every running hestiad so cargo can overwrite the locked binary and the
# next client auto-spawns the fresh build. Hard stop, matching build.sh's POSIX
# pkill: supervised workloads run in their own process groups and survive it.
function Stop-Hestia {
  if (-not (Get-Process hestiad -ErrorAction SilentlyContinue)) { return }
  Get-Process hestiad -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
  for ($i = 0; $i -lt 20 -and (Get-Process hestiad -ErrorAction SilentlyContinue); $i++) {
    Start-Sleep -Milliseconds 250
  }
}

$scripts = @{
  build    = "scripts/build.sh"
  run      = "scripts/run.sh"
  dev      = "scripts/dev.sh"
  sidecars = "scripts/sidecars.sh"
  package  = "scripts/package.sh"
  clean    = "scripts/clean.sh"
}

if (-not $Command -or -not $scripts.ContainsKey($Command)) {
  Write-Host "usage: win.ps1 <$($scripts.Keys -join '|')> [args...]"
  if ($Command) { exit 1 } else { exit 0 }
}

# A build (and the dev loop, which builds on entry) must not race a running
# daemon: stop every instance first so the overwrite succeeds and the next
# spawn is the new binary.
if ($Command -eq "build" -or $Command -eq "dev") { Stop-Hestia }

# The Windows half of dev.sh's isolation: pin the dev-only daemon endpoint (a
# named pipe rather than dev.sh's unix socket path), and drop mise's PATH
# snapshot so its prompt hook in the dev subshell re-baselines from the
# stripped PATH instead of resurrecting the installed hestia.
if ($Command -eq "dev") {
  if (-not $env:HESTIA_SOCK) { $env:HESTIA_SOCK = '\\.\pipe\hestia-hestiad-dev' }
  Get-ChildItem Env:__MISE_* -ErrorAction SilentlyContinue |
    ForEach-Object { Remove-Item "Env:$($_.Name)" }

  # Bare `dev` is the interactive loop: build, then open a PowerShell subshell
  # with the dev binaries on PATH. Forms with args (--desktop, one-shot CLI)
  # keep forwarding to dev.sh, which owns the Tauri/one-shot logic.
  if (-not $Rest) {
    Write-Host "Building daemon + CLI (debug)"
    cargo build -p daemon -p cli
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $bindir = Join-Path (Get-Location) "target\debug"
    # Drop any PATH entry that carries an installed hestia, then put the dev
    # build first, so this subshell never drives an installed daemon.
    $env:PATH = (($env:PATH -split ';') | Where-Object {
        $_ -and -not (Test-Path (Join-Path $_ 'hestia.exe')) -and
        -not (Test-Path (Join-Path $_ 'hestiad.exe'))
      }) -join ';'
    $env:PATH = "$bindir;$env:PATH"
    $env:HESTIA_DEV = "1"

    $ps = Get-Command pwsh -ErrorAction SilentlyContinue
    if ($ps) { $ps = $ps.Source } else { $ps = (Get-Command powershell).Source }

    Write-Host "hestia + hestiad on PATH ($bindir). 'exit' to leave."
    try {
      & $ps -NoLogo
    } finally {
      & "$bindir\hestia.exe" daemon stop *> $null
    }
    exit 0
  }
}

$bash = Find-Bash
& $bash $scripts[$Command] @Rest
exit $LASTEXITCODE
