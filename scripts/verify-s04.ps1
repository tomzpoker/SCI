$root=Split-Path -Parent $PSScriptRoot
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'scripts\verify-s03-0306-s04.ps1')
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
Write-Host 'S04_ONLY=PASS'
