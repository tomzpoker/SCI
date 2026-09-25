[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
$required=@('scripts/bootstrap.ps1','scripts/migrate.ps1','scripts/upgrade.ps1','scripts/rollback-release.ps1','scripts/release.ps1','scripts/git-sync.ps1','scripts/verify-git-hygiene.ps1','README.md','.gitignore','Cargo.toml','Cargo.lock')
foreach($p in $required){if(-not(Test-Path $p)){throw "Fichier release absent: $p"}}
$cargo=Get-Content Cargo.toml -Raw
if($cargo -notmatch 'version = "0\.5\.0"'){throw 'Version Cargo != 0.5.0'}
$lock=Get-Content Cargo.lock -Raw
if($lock -notmatch 'name = "sci-family-pilot"\s+version = "0\.5\.0"'){throw 'Version Cargo.lock != 0.5.0'}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'verify-git-hygiene.ps1')
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
Write-Host 'S18_RELEASE_STATIC_GATE=PASS' -ForegroundColor Green
