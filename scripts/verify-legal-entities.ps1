[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$failures = [System.Collections.Generic.List[string]]::new()

function Require-File([string]$RelativePath) {
    if (-not (Test-Path -LiteralPath (Join-Path $Root $RelativePath) -PathType Leaf)) {
        [void]$failures.Add("fichier absent: $RelativePath")
    }
}
function Require-Text([string]$RelativePath, [string]$Pattern, [string]$Label) {
    $path = Join-Path $Root $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return }
    $text = Get-Content -LiteralPath $path -Raw
    if ($text -notmatch $Pattern) { [void]$failures.Add("$Label absent dans $RelativePath") }
}

Require-File 'migrations/0006_legal_entities.sql'
Require-File 'src/domain.rs'
Require-File 'src/legal_entities.rs'
Require-File 'src/lib.rs'
Require-File 'src/ui.rs'
Require-File 'versions/S02_US-0201.md'
Require-File 'S02_US-0201.md'

Require-Text 'src/lib.rs' 'pub\s+mod\s+legal_entities;' 'module legal_entities'
Require-Text 'src/domain.rs' 'pub\s+struct\s+LegalEntityItem' 'LegalEntityItem'
Require-Text 'src/legal_entities.rs' 'pub\s+async\s+fn\s+list_legal_entities' 'list_legal_entities'
Require-Text 'src/legal_entities.rs' 'pub\s+async\s+fn\s+create_legal_entity' 'create_legal_entity'
Require-Text 'src/legal_entities.rs' 'pub\s+async\s+fn\s+update_legal_entity' 'update_legal_entity'
Require-Text 'src/legal_entities.rs' 'pub\s+async\s+fn\s+set_legal_entity_active' 'set_legal_entity_active'
Require-Text 'src/ui.rs' 'Page::LegalEntities' 'navigation entités'
Require-Text 'src/ui.rs' 'fn\s+LegalEntitiesPage' 'UI entités'

$migration = Get-Content -LiteralPath (Join-Path $Root 'migrations/0006_legal_entities.sql') -Raw
$destructive = '(?im)^\s*(DROP\s+(TABLE|COLUMN|SCHEMA|INDEX)|TRUNCATE\s+TABLE|DELETE\s+FROM)\b'
if ($migration -match $destructive) {
    [void]$failures.Add('opération SQL destructive détectée dans 0006_legal_entities.sql')
}
foreach ($required in @('legal_entities','legal_entity_bank_accounts','legal_entity_id','uq_legal_entities_workspace_siren','fk_scis_legal_entity','fk_audit_events_legal_entity')) {
    if ($migration -notmatch [regex]::Escape($required)) { [void]$failures.Add("élément SQL manquant: $required") }
}

if ($failures.Count -gt 0) {
    Write-Host 'LEGAL_ENTITIES_STATUS=FAIL'
    $failures | ForEach-Object { Write-Host "- $_" }
    exit 1
}

Write-Host 'LEGAL_ENTITIES_STATUS=PASS'
Write-Host 'checks=files,domain,server-functions,ui,migration-safety'
exit 0
