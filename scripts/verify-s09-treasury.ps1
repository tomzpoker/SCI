$ErrorActionPreference='Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
$script:checks=@()
function Check($n,$ok){$script:checks += [pscustomobject]@{Name=$n;Status=$(if($ok){'PASS'}else{'FAIL'})}}
$schema=Get-Content '.\migrations\0019_treasury_forecast.sql' -Raw
$src=Get-Content '.\src\treasury.rs' -Raw
$ui=Get-Content '.\src\ui.rs' -Raw
$lib=Get-Content '.\src\lib.rs' -Raw
Check 'Migration 0019' (Test-Path '.\migrations\0019_treasury_forecast.sql')
foreach($t in @('treasury_recurring_patterns','treasury_forecast_events','treasury_forecast_snapshots','treasury_investments','treasury_investment_scenarios','treasury_investment_scenario_items')){Check "Table $t" $schema.Contains("CREATE TABLE IF NOT EXISTS $t")}
foreach($c in @('FIXED','VARIABLE','SEASONAL','PUNCTUAL','PROBABLY_RECURRING','UNKNOWN')){Check "Récurrence $c" $schema.Contains("'$c'")}
foreach($h in @(1,2,3,6,9,12,24,36,60,120)){Check "Horizon $h" $schema.Contains("CHECK (horizon_months IN (1,2,3,6,9,12,24,36,60,120))")}
foreach($q in @('CERTAIN','PROBABLE','HYPOTHESIS','SCENARIO')){Check "Qualification $q" $schema.Contains("'$q'")}
foreach($s in @('SCHEDULED','PENDING','AWAITING_BANK_MATCH','MATCHED','COMPLETED','CANCELLED')){Check "Etat $s" $schema.Contains("'$s'")}
foreach($c in @('TRAVAUX','EQUIPEMENT','RENOVATION','COPROPRIETE','SECURITE','MISE_AUX_NORMES','RENOUVELLEMENT')){Check "Investissement $c" $schema.Contains("'$c'")}
Check 'Prévision distincte du paiement' ($src.Contains('create_treasury_hypothesis') -and $src.Contains('materialize_treasury_forecast_event'))
Check 'Traductions UI' ($src.Contains('qualification_fr') -and $src.Contains('state_fr') -and $src.Contains('category_fr'))
Check 'Scénarios' ($src.Contains('create_investment_scenario') -and $src.Contains('summarize_investment_scenario'))
Check 'Route UI Trésorerie' ($ui.Contains('Page::Treasury=>rsx!{TreasuryPage{refresh}}'))
Check 'Module exporté' ($lib.Contains('pub mod treasury;'))
Check 'Pas de DROP/TRUNCATE' (-not ($schema -match '(?i)\b(DROP TABLE|TRUNCATE)\b'))
$failed=@($script:checks|Where-Object Status -eq 'FAIL')
$script:checks|ForEach-Object{"$($_.Status) | $($_.Name)"}
if($failed.Count){throw "S09_TREASURY_GATE=FAIL ($($failed.Count))"}
'S09_TREASURY_GATE=PASS'
