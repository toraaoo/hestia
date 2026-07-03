# win.ps1 — the scripts/ dev flow on Windows.
#
#   scripts\win.ps1 build [--release] [target...]
#   scripts\win.ps1 run <daemon|cli|tray|desktop> [args...]
#   scripts\win.ps1 configure [--release] [-- <extra cmake args>]
#   scripts\win.ps1 clean [dev|release|all]
#   scripts\win.ps1 package [args...]
#
# Enters the Visual Studio x64 developer environment (MSVC + CMake + Ninja),
# then delegates to the matching bash script via Git Bash — the same recipe CI
# uses (ilammy/msvc-dev-cmd + bash steps).

param(
    [Parameter(Mandatory, Position = 0)]
    [ValidateSet("build", "run", "configure", "clean", "package")]
    [string]$Command,

    [Parameter(ValueFromRemainingArguments)]
    [string[]]$Rest
)

$ErrorActionPreference = "Stop"

if (-not $env:VSCMD_VER) {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) { throw "Visual Studio not found ($vswhere missing)" }
    $vsRoot = & $vswhere -latest -products * `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationPath
    if (-not $vsRoot) { throw "No Visual Studio installation with the C++ toolset found" }
    Import-Module (Join-Path $vsRoot "Common7\Tools\Microsoft.VisualStudio.DevShell.dll")
    Enter-VsDevShell -VsInstallPath $vsRoot -SkipAutomaticLocation -DevCmdArguments "-arch=x64" | Out-Null
}

$gitRoot = Split-Path (Split-Path (Get-Command git -ErrorAction Stop).Source)
$bash = Join-Path $gitRoot "bin\bash.exe"
if (-not (Test-Path $bash)) { throw "Git Bash not found at $bash" }

$script = (Join-Path $PSScriptRoot "$Command.sh") -replace '\\', '/'
& $bash $script @Rest
exit $LASTEXITCODE
