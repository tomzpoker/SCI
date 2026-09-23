$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
$env:Path = "C:\msys64\ucrt64\bin;$env:USERPROFILE\.cargo\bin;$env:Path"

& "$PSScriptRoot\doctor.ps1"
if (Get-Command docker -ErrorAction SilentlyContinue) {
  docker compose up -d
  docker compose ps
}

cargo check --features server
cargo test --all-targets
Write-Host 'SCI Manager technical gate: PASS' -ForegroundColor Green
