[CmdletBinding()]
param([Parameter(Mandatory)][string]$BackupId,[switch]$Apply)
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
if(-not $Apply){throw 'Rollback exige -Apply et un BackupId explicite.'}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'recovery.ps1') -Action Rollback -BackupId $BackupId -Apply
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
Write-Host 'ROLLBACK_RELEASE_STATUS=PASS' -ForegroundColor Green
