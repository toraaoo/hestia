$ErrorActionPreference = "Stop"

$env:HESTIA_DATA_DIR = if ($env:HESTIA_DATA_DIR) { $env:HESTIA_DATA_DIR } else { "$(Get-Location)/.hestia" }
$env:PATH = "$env:PATH;$(Get-Location)/dist"
Write-Host "Using data dir: $env:HESTIA_DATA_DIR"

go build -o dist/hestia.exe ./cmd/hestia
go build -o dist/hestiad.exe ./cmd/hestiad

& ./dist/hestia.exe @args
