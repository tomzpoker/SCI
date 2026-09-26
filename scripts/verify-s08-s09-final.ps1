$ErrorActionPreference='Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
& .\scripts\verify-s08-banking.ps1
& .\scripts\verify-s09-treasury.ps1
$files=Get-ChildItem .\migrations -Filter '*.sql' | Sort-Object Name
$names=$files.Name
if($names.Count -lt 19){throw "Nombre de migrations inférieur à 19 : $($names.Count)"}
if(-not (Test-Path '.\src\banking.rs')){throw 'src/banking.rs absent'}
if(-not (Test-Path '.\src\treasury.rs')){throw 'src/treasury.rs absent'}
'MANIFEST_S08_S09_FINAL=PASS'
