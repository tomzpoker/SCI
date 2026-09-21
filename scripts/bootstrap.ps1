$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..
if (-not (Test-Path .env)) { Copy-Item .env.example .env }
docker compose up -d
dx serve --web --features server
