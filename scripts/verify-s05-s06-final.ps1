$ErrorActionPreference='Stop'
& (Join-Path $PSScriptRoot 'verify-s05-documents-ocr.ps1')
& (Join-Path $PSScriptRoot 'verify-s06-leases.ps1')
$root=Split-Path $PSScriptRoot -Parent
$lib=Get-Content (Join-Path $root 'src\lib.rs') -Raw
foreach($x in @('pub mod documents;','pub mod leases;','pub mod workflow;','pub mod idempotency;')){if($lib -notmatch [regex]::Escape($x)){throw "Missing lib module $x"}}
Write-Host 'S05_S06_FINAL_GATE=PASS'
