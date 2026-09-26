$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$required = @(
  "migrations\0009_entity_change_history.sql",
  "src\history.rs",
  "src\domain.rs",
  "src\legal_entities.rs",
  "src\sarl_activities.rs",
  "src\server.rs",
  "src\ui.rs",
  "src\lib.rs",
  "S02_US-0204.md",
  "S02_US-0204_README.md",
  "versions\S02_US-0204.md"
)
foreach($rel in $required){ if(!(Test-Path (Join-Path $root $rel))){ throw "Missing: $rel" } }
$sql = Get-Content (Join-Path $root "migrations\0009_entity_change_history.sql") -Raw
foreach($bad in @("DROP TABLE","DROP COLUMN","DROP SCHEMA","TRUNCATE","DELETE FROM")){ if($sql -match [regex]::Escape($bad)){ throw "Forbidden destructive SQL: $bad" } }
foreach($needle in @("effective_at","recorded_at","author","reason","before_state","after_state","legal_entity_id")){ if($sql -notmatch [regex]::Escape($needle)){ throw "Missing history field: $needle" } }
$history = Get-Content (Join-Path $root "src\history.rs") -Raw
foreach($needle in @("record_entity_change","snapshot_legal_entity","snapshot_activity_set","list_change_history")){ if($history -notmatch [regex]::Escape($needle)){ throw "Missing history function: $needle" } }
Write-Host "CHANGE_HISTORY_STATIC_GATE=PASS"
