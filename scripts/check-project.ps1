$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)
$env:Path="C:\msys64\ucrt64\bin;$env:USERPROFILE\.cargo\bin;$env:Path"
docker compose up -d
docker compose ps
cargo check --features server
Write-Host "SCI Family Pilot smoke check: PASS" -ForegroundColor Green
