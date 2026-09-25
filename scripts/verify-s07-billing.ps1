$ErrorActionPreference='Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
$script:checks=@()
function Check($name,$ok,$detail=''){ $script:checks += [pscustomobject]@{Name=$name;Status=$(if($ok){'PASS'}else{'FAIL'});Detail=$detail} }
Check 'Migration 0017 présente' (Test-Path '.\migrations\0017_billing_invoicing.sql')
Check 'Module billing présent' (Test-Path '.\src\billing.rs')
$billing=Get-Content '.\src\billing.rs' -Raw
$schema=Get-Content '.\migrations\0017_billing_invoicing.sql' -Raw
$ui=Get-Content '.\src\ui.rs' -Raw
$lib=Get-Content '.\src\lib.rs' -Raw
foreach($s in @('DRAFT','VALIDATED','ISSUED','PAID_PARTIAL','PAID','OVERDUE','CANCELLED','CREDITED')){ Check "Etat $s" ($schema.Contains("'$s'")) }
Check 'Statuts UI français' ($billing.Contains('Brouillon') -and $billing.Contains('Validée') -and $billing.Contains('Partiellement payée') -and $billing.Contains('Créditée'))
Check 'Génération loyers idempotente' ($billing.Contains('S07:RENT:') -and $schema.Contains('uq_invoices_entity_generation_key_s07'))
Check 'Charges contractuelles' ($billing.Contains('append_fixed_charge_lines') -and $schema.Contains('billing_charge_actuals'))
Check 'Régularisations' ($billing.Contains('calculate_charge_regularization') -and $schema.Contains('billing_regularizations'))
Check 'Taxes récupérables' ($billing.Contains('lease_recoverable_taxes') -and $billing.Contains('billing_tax_assessments'))
Check 'Avoirs traçables' ($billing.Contains('create_credit_note') -and $schema.Contains('source_invoice_id') -and $schema.Contains('billing_invoice_state_history'))
Check 'Brouillon PDF marqué' ($billing.Contains('BROUILLON — SANS EFFET EXTERNE') -and $billing.Contains('pdf_draft_watermark'))
Check 'Pas de suppression physique des factures' ((Get-Content '.\src\server.rs' -Raw).Contains("UPDATE invoices SET status='CANCELLED'") -and -not ((Get-Content '.\src\server.rs' -Raw) -match 'DELETE FROM invoices'))
Check 'Plateforme e-facturation interchangeable' ($schema.Contains('billing_einvoice_connections') -and $schema.Contains('uq_billing_einvoice_active_s07') -and $billing.Contains('set_einvoice_provider'))
Check 'Isolation plateforme par entité' ($schema.Contains('uq_billing_einvoice_connections_id_entity_s07') -and $schema.Contains('FOREIGN KEY (connection_id, legal_entity_id)'))
Check 'Historique e-facturation' ($schema.Contains('billing_einvoice_events') -and $billing.Contains('prepare_einvoice'))
Check 'Route UI Facturation S07' ($ui.Contains('Page::Billing=>rsx!{BillingManagementPage{refresh}}'))
Check 'Module billing exporté' ($lib.Contains('pub mod billing;'))
$failed=@($script:checks | Where-Object Status -eq 'FAIL')
$script:checks | ForEach-Object { "$($_.Status) | $($_.Name)$(if($_.Detail){" | $($_.Detail)"})" }
if($failed.Count -gt 0){ throw "S07_BILLING_GATE=FAIL ($($failed.Count) contrôles)" }
'S07_BILLING_GATE=PASS'
