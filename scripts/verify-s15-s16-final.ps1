$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
$m=Get-ChildItem (Join-Path $Root 'migrations') -Filter '*.sql' -File | Where-Object { $_.Name -match '^\d+_.*\.sql$' } | Sort-Object Name
$v=@($m | ForEach-Object { [int]([regex]::Match($_.Name,'^\d+').Value) })
$baseline=@($v | Where-Object { $_ -le 28 })
if(($baseline -join ',') -ne ((1..28) -join ',')){throw 'Baseline migrations S15-S16 attendue: 0001..0028'}
foreach($f in 'migrations/0027_ux_zero_saisie.sql','migrations/0028_security_recovery.sql'){
  if(-not(Test-Path(Join-Path $Root $f))){throw "Fichier absent: $f"}
  $s=Get-Content(Join-Path $Root $f)-Raw
  if($s -match '(?i)DROP\s+TABLE|TRUNCATE\s+|DROP\s+COLUMN'){throw "SQL destructive dans $f"}
}
$ux=Get-Content(Join-Path $Root 'migrations/0027_ux_zero_saisie.sql')-Raw
$sec=Get-Content(Join-Path $Root 'migrations/0028_security_recovery.sql')-Raw
$required=@('SCI','REGIME_FISCAL','TVA','BIENS','LOTS','LOCATAIRES','BAUX','BANQUE','DOCUMENTS','ASSOCIES','IMPORTS','AUTOMATISATIONS','NOTIFICATIONS','IA')
foreach($x in $required){if($ux -notmatch [regex]::Escape("'$x'")){throw "Onboarding absent: $x"}}
foreach($role in 'OWNER','MANAGER','ACCOUNTANT','VIEWER','AI_AGENT','SYSTEM'){if($sec -notmatch [regex]::Escape("('$role')")){throw "Role absent: $role"}}
foreach($f in 'src/ux.rs','src/security.rs','src/assistant/tools.rs','src/assistant/control.rs','src/infrastructure.rs','src/entity_scope.rs','scripts/doctor.ps1','scripts/recovery.ps1','scripts/sci.ps1'){if(-not(Test-Path(Join-Path $Root $f))){throw "Fichier absent: $f"}}
$tools=Get-Content(Join-Path $Root 'src/assistant/tools.rs')-Raw
$codes=@('get_cash_balance','get_real_bank_balance','get_forecast','get_unpaid_rents','get_upcoming_deadlines','calculate_rent_revision','calculate_vat','prepare_vat_return','prepare_invoice','prepare_reminder','search_documents','get_lease','get_tenant','get_property','simulate_tax','create_draft','request_user_validation')
foreach($x in $codes){if($tools -notmatch [regex]::Escape('"'+$x+'"')){throw "AI tool absent: $x"}}
Write-Host 'S15_S16_GATE=PASS'
Write-Host 'MIGRATIONS_BASELINE=0001..0028'
Write-Host ('MIGRATIONS_CURRENT=0001..{0:D4}' -f $v[-1])
Write-Host 'DESTRUCTIVE_S15_S16=NONE'
