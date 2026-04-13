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
  dotnet publish $projectPath -c Release -r $Rid --self-contained true -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true -p:PublishTrimmed=false -p:Version=$Version -p:InformationalVersion=$Version -o $outDir
  if ($LASTEXITCODE -ne 0) { throw "dotnet publish failed: $projectPath" }
}

$tuiOut = Join-Path $stageDir 'tui'
$desktopOut = Join-Path $stageDir 'desktop'

Publish-Project (Join-Path $repoRoot 'src/Hestia.Tui/Hestia.Tui.csproj') $tuiOut
Publish-Project (Join-Path $repoRoot 'src/Hestia.Desktop/Hestia.Desktop.csproj') $desktopOut

# Flatten into the stage root.
Copy-Item -Force -Path (Join-Path $tuiOut '*') -Destination $stageDir
Copy-Item -Force -Path (Join-Path $desktopOut '*') -Destination $stageDir

Remove-Item -Recurse -Force $tuiOut
Remove-Item -Recurse -Force $desktopOut

# Strip debug symbols from release artifacts.
Get-ChildItem -Path $stageDir -Filter '*.pdb' -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue

# Ensure the installed command is `hestia`.
if ($Rid -eq 'win-x64') {
  $exe = Join-Path $stageDir 'hestia.exe'
  if (-not (Test-Path $exe)) { throw "Expected $exe in staging output." }
} else {
  $bin = Join-Path $stageDir 'hestia'
  if (-not (Test-Path $bin)) { throw "Expected $bin in staging output." }
  chmod +x $bin | Out-Null
}
