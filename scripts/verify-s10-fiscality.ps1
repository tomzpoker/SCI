$ErrorActionPreference='Stop'; Set-Location (Split-Path -Parent $PSScriptRoot)
$required=@('migrations/0020_fiscality.sql','src/fiscal.rs','S10_FINAL.md')
foreach($p in $required){if(!(Test-Path $p)){throw "FICHIER_MANQUANT=$p"}}
if((Select-String -Path 'migrations/0020_fiscality.sql' -Pattern 'DROP TABLE|TRUNCATE TABLE' -Quiet)){throw 'MIGRATION_DESTRUCTIVE'}
if(!(Select-String -Path 'src/fiscal.rs' -Pattern 'COLLECTION|VALIDATION_REQUIRED|SIMULATION|POTENTIALLY_APPLICABLE|2072-S' -Quiet)){throw 'S10_MARKERS_MISSING'}
Write-Host 'S10_STATIC_GATE=PASS' -ForegroundColor Green
