$ErrorActionPreference='Stop'; Set-Location (Split-Path -Parent $PSScriptRoot)
$required=@('migrations/0022_document_generation.sql','src/generation/mod.rs','src/generation/pdf.rs','S12_FINAL.md')
foreach($p in $required){if(!(Test-Path $p)){throw "FICHIER_MANQUANT=$p"}}
if((Select-String -Path 'migrations/0022_document_generation.sql' -Pattern 'DROP TABLE|TRUNCATE TABLE' -Quiet)){throw 'MIGRATION_DESTRUCTIVE'}
if(!(Select-String -Path 'src/generation/mod.rs' -Pattern 'create_generated_document|generate_lease_document|generate_generated_document_pdf|validation_required' -Quiet)){throw 'S12_MARKERS_MISSING'}
if(!(Select-String -Path 'src/generation/pdf.rs' -Pattern 'A4|WinAnsi|write_pdf|jpeg_info' -Quiet)){throw 'PDF_MARKERS_MISSING'}
Write-Host 'S12_STATIC_GATE=PASS' -ForegroundColor Green
