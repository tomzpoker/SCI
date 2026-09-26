[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
$env:SCI_RUN_DB_TESTS='1'
$env:SCI_TEST_AUTH='1'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'migrate.ps1')
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
cargo test --test e2e_cycle --features server,test-auth
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
cargo test --test integration --features server,test-auth
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
Write-Host 'S19_E2E_STATUS=PASS' -ForegroundColor Green
