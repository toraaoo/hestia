# win.ps1 — run the scripts/ dev flow on Windows via Git Bash.
#
# A thin forwarder: every verb maps to the matching scripts/*.sh, so the shell
# scripts stay the single source of truth (no duplicated packaging logic) and
# the terminal-first test loop — `dev` — is available on Windows too.
#
# Requires Git for Windows (provides bash.exe). WSL's bash is intentionally
# skipped: it runs a Linux userland that can't see the Windows cargo target.
#
#   scripts\win.ps1 dev      [--desktop] [hestia args...]   # build + subshell / one-shot CLI
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

$bash = Find-Bash
& $bash $scripts[$Command] @Rest
exit $LASTEXITCODE
