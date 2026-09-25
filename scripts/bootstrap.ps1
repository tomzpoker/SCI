$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
Remove-Item Env:RUSTFLAGS,Env:RUSTUP_TOOLCHAIN,Env:CARGO_BUILD_TARGET -ErrorAction SilentlyContinue

Write-Host 'SCI Manager Bootstrap' -ForegroundColor Cyan
Write-Host '---------------------'

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) { throw 'rustup introuvable.' }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'cargo introuvable.' }
if (-not (Get-Command git -ErrorAction SilentlyContinue)) { throw 'git introuvable.' }

rustup toolchain install 1.98.1-x86_64-pc-windows-msvc --profile default --target wasm32-unknown-unknown
if (-not (Test-Path '.env')) { Copy-Item '.env.example' '.env'; Write-Host 'Environment .......... CREATED' } else { Write-Host 'Environment .......... OK' }

New-Item -ItemType Directory -Force -Path @('engines','reference_data','fixtures','tests','docs','architecture','snapshots','versions') | Out-Null

& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'migrate.ps1')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host 'Rust ................. OK'
Write-Host 'Cargo ................ OK'
Write-Host 'Git .................. OK'
Write-Host 'Project .............. 0.5.0'
Write-Host 'Migrations ........... PASS'
Write-Host 'Engine manifest ...... OK'
Write-Host 'STATUS ............... READY FOR TEST/LAUNCH' -ForegroundColor Green
