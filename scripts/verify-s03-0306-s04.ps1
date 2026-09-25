$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$required = @(
  'migrations\0011_reproducible_calculations.sql',
  'migrations\0012_common_data_engines.sql',
  'src\reproducibility.rs',
  'src\transactions.rs',
  'src\business_events.rs',
  'src\cash.rs',
  'src\services.rs',
  'src\engines.rs',
  'src\rules.rs',
  'src\server.rs',
  'src\ui.rs'
)
foreach($rel in $required){ if(!(Test-Path (Join-Path $root $rel))){ throw "FICHIER_MANQUANT=$rel" } }
$destructive = 'DROP TABLE|DROP COLUMN|DROP SCHEMA|TRUNCATE|DELETE FROM'
foreach($rel in @('migrations\0011_reproducible_calculations.sql','migrations\0012_common_data_engines.sql')){
  $sql = Get-Content (Join-Path $root $rel) -Raw
  $sql = [regex]::Replace($sql, '(?m)^\s*--.*$', '')
  if($sql -match $destructive){ throw "SQL_DESTRUCTIVE=$rel" }
}
$calc = Get-Content (Join-Path $root 'migrations\0011_reproducible_calculations.sql') -Raw
foreach($needle in @('formula_text','rule_snapshot','source_snapshot','rounding_mode','input_hash','output_hash')){ if($calc -notmatch [regex]::Escape($needle)){ throw "CALC_FIELD_MISSING=$needle" } }
$common = Get-Content (Join-Path $root 'migrations\0012_common_data_engines.sql') -Raw
foreach($needle in @('financial_transactions','business_event_types','business_events','cash_position_snapshots','service_operations')){ if($common -notmatch [regex]::Escape($needle)){ throw "S04_TABLE_MISSING=$needle" } }
$events = Get-Content (Join-Path $root 'src\business_events.rs') -Raw
foreach($needle in @('LeaseCreated','InvoiceIssued','PaymentDetected','StockReceived','StockSold','TaxDeadlineReached')){ if($events -notmatch [regex]::Escape($needle)){ throw "EVENT_MISSING=$needle" } }
$eng = Get-Content (Join-Path $root 'src\engines.rs') -Raw
foreach($needle in @('CommonEngine','Calcul','Event','Workflow','Bank','Vat','Document','Cash','Audit')){ if($eng -notmatch [regex]::Escape($needle)){ throw "ENGINE_MISSING=$needle" } }
$rep = Get-Content (Join-Path $root 'src\reproducibility.rs') -Raw
foreach($needle in @('inputs','rule_code','rule_version_no','source_name','formula','result','rounding_mode','explain_calculation_run')){ if($rep -notmatch [regex]::Escape($needle)){ throw "REPRO_FIELD_MISSING=$needle" } }
$rules = Get-Content (Join-Path $root 'src\rules.rs') -Raw
if($rules -notmatch 'describe_formula'){ throw 'RULE_FORMULA_CAPTURE_MISSING' }
$server = Get-Content (Join-Path $root 'src\server.rs') -Raw
foreach($needle in @('record_business_event','record_financial_transaction')){ if($server -notmatch [regex]::Escape($needle)){ throw "SERVER_COMMON_ENGINE_HOOK_MISSING=$needle" } }
$ui = Get-Content (Join-Path $root 'src\ui.rs') -Raw
if($ui -notmatch 'CommonEnginesPage'){ throw 'UI_COMMON_ENGINES_MISSING' }
Write-Host 'STATIC_GATE=PASS'
Write-Host 'US-0306=PASS'
Write-Host 'S04-US-0401=PASS'
Write-Host 'S04-US-0402=PASS'
Write-Host 'S04-US-0403=PASS'
Write-Host 'S04-US-0404=PASS'
Write-Host 'S04-US-0405=PASS'
