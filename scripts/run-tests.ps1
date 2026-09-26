[CmdletBinding()]
param([switch]$Database,[switch]$E2E)
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
if(-not(Get-Command cargo -ErrorAction SilentlyContinue)){throw 'cargo introuvable.'}
$feature='server'
if($Database -or $E2E){$feature='server,test-auth'}
Write-Host "TEST_FEATURES=$feature"
if($Database -or $E2E){$env:SCI_TEST_AUTH='1'}
if($Database){$env:SCI_RUN_DB_TESTS='1'}
cargo test --lib --features $feature
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
cargo test --test pure_hardening --features $feature
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
cargo test --test snapshot_documents --features $feature
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
cargo test --test fiscal_fixtures --features $feature
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
cargo test --test workflow_s17 --features $feature
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
cargo test --test resilience --features $feature
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
cargo test --test migration_manifest --features $feature
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
cargo test --test security_hardening --features $feature
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
cargo test --test e2e_cycle --features $feature
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
if($Database -or $E2E){cargo test --test integration --features $feature;if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}}
Write-Host 'TEST_SUITE_STATUS=PASS' -ForegroundColor Green
