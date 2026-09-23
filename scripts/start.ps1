$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
Remove-Item Env:RUSTFLAGS,Env:RUSTUP_TOOLCHAIN,Env:CARGO_BUILD_TARGET -ErrorAction SilentlyContinue

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) { throw 'rustup introuvable.' }
if (-not (Get-Command docker -ErrorAction SilentlyContinue)) { throw 'Docker introuvable. Installez Docker Desktop puis relancez start.ps1.' }
docker info *> $null; if ($LASTEXITCODE -ne 0) { throw 'Docker Desktop is not running.' }

if (-not (Test-Path '.env')) { Copy-Item '.env.example' '.env'; Write-Host 'Environment .......... CREATED' -ForegroundColor Green } else { Write-Host 'Environment .......... OK' -ForegroundColor Green }

rustup toolchain install 1.98.1-x86_64-pc-windows-msvc --profile default --target wasm32-unknown-unknown

Write-Host 'PostgreSQL ............ STARTING' -ForegroundColor Cyan
docker compose up -d --wait postgres
if ($LASTEXITCODE -ne 0) { throw 'PostgreSQL startup failed.' }
Write-Host 'PostgreSQL ............ READY' -ForegroundColor Green

if (-not (Get-Command dx -ErrorAction SilentlyContinue) -or ((dx --version) -notmatch 'dioxus 0\.7\.10')) { Write-Host 'Dioxus 0.7.10 ........ INSTALLING' -ForegroundColor Cyan; cargo install dioxus-cli --version 0.7.10 --locked --force }

Write-Host 'SCI Family Pilot ...... STARTING' -ForegroundColor Cyan
dx serve --web
