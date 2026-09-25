$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot
$required=@(
  'migrations/0010_reference_rule_engine.sql',
  'src/rules.rs',
  'src/lib.rs',
  'src/ui.rs',
  'S03.md',
  'S03_US-0301.md',
  'S03_US-0302.md',
  'S03_US-0303.md',
  'S03_US-0304.md',
  'S03_US-0305.md'
)
foreach($f in $required){if(!(Test-Path (Join-Path $root $f))){throw "Fichier manquant: $f"}}
$sql=Get-Content (Join-Path $root 'migrations/0010_reference_rule_engine.sql') -Raw
foreach($bad in @('DROP TABLE','DROP COLUMN','DROP SCHEMA','TRUNCATE TABLE','DELETE FROM')){if($sql -match [regex]::Escape($bad)){throw "Motif destructif interdit: $bad"}}
foreach($needle in @('versioned_references','rule_definitions','rule_versions','rule_calculation_runs','valid_from','source_reference_id','PUBLISHED','RETIRED','RuleVersion publiée est immuable')){if($sql -notmatch [regex]::Escape($needle)){throw "Contrat absent: $needle"}}
$rules=Get-Content (Join-Path $root 'src/rules.rs') -Raw
foreach($needle in @('resolve_rule_version','create_calculation_run','replay_calculation_run','evaluate_definition','MULTIPLY_BPS','source_reference_id')){if($rules -notmatch [regex]::Escape($needle)){throw "Implémentation absente: $needle"}}
$ui=Get-Content (Join-Path $root 'src/ui.rs') -Raw
foreach($needle in @('Page::Rules','RulesPage')){if($ui -notmatch [regex]::Escape($needle)){throw "UI absente: $needle"}}
Write-Host 'STATIC_GATE=PASS'
Write-Host 'REFERENCE_RULE_ENGINE_GATE=PASS'
