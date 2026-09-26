[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$required = @(
  'migrations\0008_entity_data_isolation.sql',
  'src\entity_scope.rs',
  'src\server.rs',
  'src\intelligence.rs',
  'src\assistant\mod.rs',
  'src\ui.rs',
  'src\lib.rs',
  'assets\main.css',
  'S02_US-0203.md',
  'versions\S02_US-0203.md'
)
$failures = @()
foreach ($rel in $required) {
  if (-not (Test-Path (Join-Path $root $rel) -PathType Leaf)) {
    $failures += "fichier absent: $rel"
  }
}

$migration = Get-Content (Join-Path $root 'migrations\0008_entity_data_isolation.sql') -Raw
foreach ($bad in @('DROP TABLE','DROP COLUMN','DROP SCHEMA','DROP INDEX','TRUNCATE','DELETE FROM')) {
  if ($migration -match "(?im)\b$([regex]::Escape($bad))\b") {
    $failures += "operation destructive interdite dans 0008: $bad"
  }
}

$scopedTables = @(
  'associates','properties','units','tenants','leases','invoices','payments','bank_transactions',
  'automation_rules','tasks','tax_deadlines','documents','forecast_snapshots','app_settings',
  'storage_locations','llm_accounts','email_accounts','document_inbox','assistant_proposals'
)
foreach ($table in $scopedTables) {
  if ($migration -notmatch "ALTER TABLE $table ADD COLUMN IF NOT EXISTS legal_entity_id UUID") {
    $failures += "colonne legal_entity_id absente pour $table"
  }
  if ($migration -notmatch "ALTER TABLE $table ALTER COLUMN legal_entity_id SET NOT NULL") {
    $failures += "legal_entity_id non obligatoire pour $table"
  }
  if ($migration -notmatch "fk_${table}_legal_entity") {
    $failures += "FK entité absente pour $table"
  }
  if ($migration -notmatch "idx_${table}_entity") {
    $failures += "index de portée absent pour $table"
  }
}

if ($migration -notmatch 'CREATE UNIQUE INDEX IF NOT EXISTS uq_units_id_legal_entity') { $failures += 'unicité paire unité/entité absente' }
if ($migration -notmatch 'CREATE UNIQUE INDEX IF NOT EXISTS uq_leases_id_legal_entity') { $failures += 'unicité paire bail/entité absente' }
if ($migration -notmatch 'fk_units_property_entity') { $failures += 'FK unité -> bien cloisonnée absente' }
if ($migration -notmatch 'fk_leases_unit_entity') { $failures += 'FK bail -> unité cloisonnée absente' }
if ($migration -notmatch 'fk_leases_tenant_entity') { $failures += 'FK bail -> locataire cloisonnée absente' }
if ($migration -notmatch 'fk_invoices_lease_entity') { $failures += 'FK facture -> bail cloisonnée absente' }
if ($migration -notmatch 'fk_payments_invoice_entity') { $failures += 'FK paiement -> facture cloisonnée absente' }
if ($migration -notmatch 'fk_tasks_rule_entity') { $failures += 'FK tâche -> règle cloisonnée absente' }
if ($migration -notmatch 'fk_inbox_storage_entity') { $failures += 'FK inbox -> stockage cloisonnée absente' }
if ($migration -match "'units','tenants'" -and $migration -match 'FOREACH target_table') { $failures += 'le parcours FK sci_id contient une table sans sci_id' }

$scope = Get-Content (Join-Path $root 'src\entity_scope.rs') -Raw
foreach ($needle in @('current_legal_entity_id','set_active_legal_entity','LEGACY_SCI_LEGAL_ENTITY_ID')) {
  if ($scope -notmatch [regex]::Escape($needle)) { $failures += "contexte entité absent: $needle" }
}

$server = Get-Content (Join-Path $root 'src\server.rs') -Raw
if ($server -notmatch 'use crate::entity_scope::current_legal_entity_id') { $failures += 'server: contexte entité non importé' }
if ($server -match 'fn sci_id\s*\(') { $failures += 'server: ancien sci_id() global encore présent' }
foreach ($fn in @('list_invoices','create_invoice_from_lease','create_payment','create_bank_transaction','list_automation_rules','list_tasks','list_documents')) {
  if ($server -notmatch "pub async fn $fn") { $failures += "server: fonction attendue absente $fn" }
}

$intel = Get-Content (Join-Path $root 'src\intelligence.rs') -Raw
if ($intel -notmatch 'current_legal_entity_id') { $failures += 'intelligence: portée entité absente' }
if ($intel -match 'WHERE sci_id=\$1|WHERE sci_id=\$[0-9]') { $failures += 'intelligence: requête encore filtrée uniquement par sci_id' }

$assistant = Get-Content (Join-Path $root 'src\assistant\mod.rs') -Raw
if ($assistant -notmatch 'legal_entity_id: Uuid') { $failures += 'assistant: contexte legal_entity_id absent' }
if ($assistant -match 'sci_id: Uuid') { $failures += 'assistant: ancien contexte sci_id détecté' }

$ui = Get-Content (Join-Path $root 'src\ui.rs') -Raw
foreach ($needle in @('EntityScopeSelector','set_active_legal_entity','Périmètre','Page::LegalEntities')) {
  if ($ui -notmatch [regex]::Escape($needle)) { $failures += "UI périmètre absente: $needle" }
}
$css = Get-Content (Join-Path $root 'assets\main.css') -Raw
if ($css -notmatch '\.scope-selector') { $failures += 'CSS du sélecteur d''entité absente' }

if ($failures.Count) {
  $failures | ForEach-Object { Write-Error $_ }
  exit 1
}

Write-Host 'ENTITY_ISOLATION_STATIC_GATE=PASS' -ForegroundColor Green
Write-Host 'migration=0008_entity_data_isolation.sql'
Write-Host 'scoped_tables=19'
Write-Host 'runtime_scope=current_legal_entity_id'
Write-Host 'next=US-0204'
