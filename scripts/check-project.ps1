[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$doctor = Join-Path $PSScriptRoot 'doctor.ps1'
if (-not (Test-Path $doctor -PathType Leaf)) { throw "Doctor script missing: $doctor" }

& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $doctor
$doctorExit = $LASTEXITCODE

if ($doctorExit -ne 0) {
    Write-Host "SCI Manager technical gate: BLOCKED (doctor exit=$doctorExit)" -ForegroundColor Red
    exit $doctorExit
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'cargo introuvable.' }
cargo check --features server
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo test --all-targets
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host 'SCI Manager technical gate: PASS' -ForegroundColor Green
exit 0
