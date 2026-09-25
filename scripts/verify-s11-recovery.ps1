$ErrorActionPreference='Stop'; Set-Location (Split-Path -Parent $PSScriptRoot)
$required=@('migrations/0021_unpaid_recovery.sql','src/collections.rs','S11_FINAL.md')
foreach($p in $required){if(!(Test-Path $p)){throw "FICHIER_MANQUANT=$p"}}
if((Select-String -Path 'migrations/0021_unpaid_recovery.sql' -Pattern 'DROP TABLE|TRUNCATE TABLE' -Quiet)){throw 'MIGRATION_DESTRUCTIVE'}
if(!(Select-String -Path 'migrations/0021_unpaid_recovery.sql' -Pattern 'guard_collection_legal_execution_s11|approval' -Quiet)){throw 'LEGAL_GUARD_MISSING'}
if(!(Select-String -Path 'src/collections.rs' -Pattern 'PAYMENT_PROMISE|FORMAL_NOTICE_PREPARATION|COURT_HUISSIER_PREPARATION' -Quiet)){throw 'S11_MARKERS_MISSING'}
Write-Host 'S11_STATIC_GATE=PASS' -ForegroundColor Green
