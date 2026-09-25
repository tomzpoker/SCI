$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
$required=@(
'migrations\0015_documents_ocr.sql',
'src\documents\workflow.rs',
'src\documents\classifier.rs',
'src\documents\extractor.rs',
'src\documents\ocr.rs',
'src\domain.rs',
'src\server.rs',
'src\ui.rs'
)
foreach($f in $required){if(!(Test-Path (Join-Path $root $f))){throw "Missing $f"}}
$sql=Get-Content (Join-Path $root 'migrations\0015_documents_ocr.sql') -Raw
$checks=@('document_storage_config','document_ocr_runs','document_extractions','document_quality_checks','document_duplicate_candidates','document_events','content_hash','archived_at','file_size_bytes')
foreach($x in $checks){if($sql -notmatch [regex]::Escape($x)){throw "Missing SQL marker $x"}}
$wf=Get-Content (Join-Path $root 'src\documents\workflow.rs') -Raw
foreach($x in @('sha256_hex','import_document_bytes','run_document_ocr','validate_document_extraction','archive_document_record','DocumentsWorkflowPage')){if($wf -notmatch [regex]::Escape($x)){throw "Missing code marker $x"}}
$server=Get-Content (Join-Path $root 'src\server.rs') -Raw
if($server -match '(?im)^\s*DELETE\s+FROM\s+documents\b'){throw 'Physical document deletion still present'}
$bad=$sql -split "`n" | Where-Object {$_ -notmatch '^\s*--' -and $_ -match '(?i)\b(DROP\s+(TABLE|COLUMN|SCHEMA|INDEX)|TRUNCATE|DELETE\s+FROM)\b'}
if($bad){throw ('Destructive SQL detected: '+($bad -join ' | '))}
Write-Host 'S05_DOCUMENTS_OCR_GATE=PASS'
