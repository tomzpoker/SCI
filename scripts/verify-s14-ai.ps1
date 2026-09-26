$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$sql=Get-Content $root\migrations4_ai_controlled.sql,$root\migrations6_ai_controlled.sql -Raw
if($sql -match '(?i)\bDROP TABLE\b|\bTRUNCATE\b'){throw 'SQL destructive détecté'}
foreach($x in 'LOCAL_LLM','EXTERNAL_LLM','DISABLED'){if($sql -notmatch $x){throw "Provider IA absent: $x"}}
$codes=@('get_cash_balance','get_real_bank_balance','get_forecast','get_unpaid_rents','get_upcoming_deadlines','calculate_rent_revision','calculate_vat','prepare_vat_return','prepare_invoice','prepare_reminder','search_documents','get_lease','get_tenant','get_property','simulate_tax','create_draft','request_user_validation')
foreach($x in $codes){if($sql -notmatch [regex]::Escape($x)){throw "Tool IA absent: $x"}}
if($codes.Count -ne 17){throw 'Catalogue IA attendu: 17 tools'}
foreach($x in 'PREFERENCE','HABIT','AUTOMATION','DOCUMENT_STYLE','UX'){if($sql -notmatch $x){throw "Mémoire autorisée absente: $x"}}
Write-Host 'S14_AI_STATIC_GATE=PASS'
