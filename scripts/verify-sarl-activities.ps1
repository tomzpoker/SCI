$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$required = @(
  "migrations\0007_sarl_activities.sql",
  "src\domain.rs",
  "src\sarl_activities.rs",
  "src\lib.rs",
  "src\ui.rs",
  "S02_US-0202.md",
  "versions\S02_US-0202.md"
)
$failures = @()
foreach ($rel in $required) {
  if (-not (Test-Path (Join-Path $root $rel))) { $failures += "fichier absent: $rel" }
}
$migration = Get-Content (Join-Path $root "migrations\0007_sarl_activities.sql") -Raw
foreach ($bad in @('DROP TABLE','DROP COLUMN','DROP SCHEMA','TRUNCATE','DELETE FROM')) {
  if ($migration -match "(?im)\b$([regex]::Escape($bad))\b") { $failures += "operation destructive interdite dans 0007: $bad" }
}
if ($migration -notmatch 'CREATE TABLE IF NOT EXISTS legal_activity_catalog') { $failures += 'catalogue activité absent' }
if ($migration -notmatch 'CREATE TABLE IF NOT EXISTS legal_entity_activities') { $failures += 'association activité absent' }
if ($migration -notmatch 'MARCHAND_DE_BIENS') { $failures += 'référence MARCHAND_DE_BIENS absente' }
if ($migration -notmatch 'GARAGE_AUTO') { $failures += 'référence GARAGE_AUTO absente' }
if ($migration -notmatch "legal_form_code = 'SARL'") { $failures += 'garde SARL absent' }
$module = Get-Content (Join-Path $root "src\sarl_activities.rs") -Raw
foreach ($fn in @('list_sarl_entities_for_activities','list_activity_catalog','list_legal_entity_activities','set_legal_entity_activity','set_primary_legal_entity_activity')) {
  if ($module -notmatch "pub async fn $fn") { $failures += "fonction absente: $fn" }
}
$ui = Get-Content (Join-Path $root "src\ui.rs") -Raw
foreach ($needle in @('SarlActivitiesPage','Page::SarlActivities','Activités SARL')) {
  if ($ui -notmatch [regex]::Escape($needle)) { $failures += "UI absente: $needle" }
}
if ($failures.Count) {
  $failures | ForEach-Object { Write-Error $_ }
  exit 1
}
Write-Host "SARL_ACTIVITIES_STATIC_GATE=PASS"
Write-Host "migration=0007_sarl_activities.sql"
Write-Host "catalog=MARCHAND_DE_BIENS,GARAGE_AUTO"
Write-Host "next=US-0203"
