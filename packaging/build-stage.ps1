param(
  [Parameter(Mandatory = $true)]
  [ValidateSet('win-x64', 'linux-x64')]
  [string] $Rid,

  [Parameter(Mandatory = $true)]
  [string] $Version
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$stageDir = Join-Path $repoRoot "artifacts/stage/$Rid"

if (Test-Path $stageDir) { Remove-Item -Recurse -Force $stageDir }
New-Item -ItemType Directory -Force -Path $stageDir | Out-Null

function Publish-Project([string] $projectPath, [string] $outDir)
{
  dotnet publish $projectPath -c Release -r $Rid --self-contained true -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true -p:PublishTrimmed=false -p:DebugType=None -p:DebugSymbols=false -p:Version=$Version -p:InformationalVersion=$Version -o $outDir
  if ($LASTEXITCODE -ne 0) { throw "dotnet publish failed: $projectPath" }
}

$tuiOut = Join-Path $stageDir 'tui'
$desktopOut = Join-Path $stageDir 'desktop'

Publish-Project (Join-Path $repoRoot 'src/Hestia.Tui/Hestia.Tui.csproj') $tuiOut
Publish-Project (Join-Path $repoRoot 'src/Hestia.Desktop/Hestia.Desktop.csproj') $desktopOut

# Fail fast if the two publish outputs would overwrite each other.
$tuiFiles = Get-ChildItem -Path $tuiOut -Recurse -File | ForEach-Object { $_.FullName.Substring($tuiOut.Length).TrimStart([IO.Path]::DirectorySeparatorChar) }
$seen = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::OrdinalIgnoreCase)
foreach ($p in $tuiFiles) { [void]$seen.Add($p) }

$collisions = @()
Get-ChildItem -Path $desktopOut -Recurse -File | ForEach-Object {
  $rel = $_.FullName.Substring($desktopOut.Length).TrimStart([IO.Path]::DirectorySeparatorChar)
  if ($seen.Contains($rel)) { $collisions += $rel }
}

if ($collisions.Count -gt 0) {
  throw "Staging collision between TUI and Desktop publish outputs: $($collisions -join ', ')"
}

# Flatten into the stage root.
Copy-Item -Force -Path (Join-Path $tuiOut '*') -Destination $stageDir
Copy-Item -Force -Path (Join-Path $desktopOut '*') -Destination $stageDir

Remove-Item -Recurse -Force $tuiOut
Remove-Item -Recurse -Force $desktopOut

# Strip debug symbols from release artifacts.
Get-ChildItem -Path $stageDir -Recurse -Filter '*.pdb' -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue

# Ensure the installed command is `hestia`.
if ($Rid -eq 'win-x64') {
  $exe = Join-Path $stageDir 'hestia.exe'
  if (-not (Test-Path $exe)) { throw "Expected $exe in staging output." }
} else {
  $bin = Join-Path $stageDir 'hestia'
  if (-not (Test-Path $bin)) { throw "Expected $bin in staging output." }
  chmod +x $bin | Out-Null
}
