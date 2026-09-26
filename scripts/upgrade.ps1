[CmdletBinding()]
param([switch]$SkipGit,[switch]$SkipTests)
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
if(-not $SkipGit -and (Get-Command git -ErrorAction SilentlyContinue)){
  git pull --ff-only
  if($LASTEXITCODE -ne 0){throw 'Git pull --ff-only a échoué.'}
}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'migrate.ps1')
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
if(-not $SkipTests){& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'run-tests.ps1');if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'doctor.ps1')
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
Write-Host 'UPGRADE_STATUS=PASS' -ForegroundColor Green
