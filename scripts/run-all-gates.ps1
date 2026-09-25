[CmdletBinding()]
param([switch]$Database)
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'verify-s17-s19-final.ps1');if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'verify-s18-release.ps1');if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'verify-s19-cycle.ps1');if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'run-tests.ps1') -Database:$Database -E2E:$Database;if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
Write-Host 'ALL_GATES_STATUS=PASS' -ForegroundColor Green
