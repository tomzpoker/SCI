$ErrorActionPreference='Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
$script:checks=@()
function Check($n,$ok){$script:checks += [pscustomobject]@{Name=$n;Status=$(if($ok){'PASS'}else{'FAIL'})}}
$schema=Get-Content '.\migrations\0018_banking_reconciliation.sql' -Raw
$src=Get-Content '.\src\banking.rs' -Raw
$ui=Get-Content '.\src\ui.rs' -Raw
$lib=Get-Content '.\src\lib.rs' -Raw
Check 'Migration 0018' (Test-Path '.\migrations\0018_banking_reconciliation.sql')
Check 'Accounts' ($schema.Contains('bank_account_profiles') -and $src.Contains('save_bank_account_management'))
Check 'CSV' ($src.Contains('import_bank_csv_management') -and $src.Contains('parse_csv_records'))
Check 'OFX' ($src.Contains('import_bank_ofx_management') -and $src.Contains('parse_ofx_records'))
Check 'PDF OCR' ($src.Contains('import_bank_pdf_ocr_management') -and $src.Contains('SCI_OCR_COMMAND'))
Check 'Champ transaction_date' $schema.Contains('transaction_date DATE')
Check 'Champ value_date' ((Get-Content '.\migrations\0001_foundation.sql' -Raw).Contains('value_date DATE'))
foreach($s in @('balance_cents','reference','source','transaction_hash','debit_credit')){Check "Champ $s" $schema.Contains("ADD COLUMN IF NOT EXISTS $s")}
foreach($s in @('MATCHED','PROBABLE','TO_VALIDATE','UNMATCHED','ANOMALY')){Check "Niveau $s" $schema.Contains("'$s'")}
foreach($s in @('FEE','INTERNAL_TRANSFER','DEPOSIT','REFUND','IMPAYE','UNKNOWN','PAYMENT_PARTIAL','PAYMENT_MULTIPLE')){Check "Nature $s" $src.Contains($s)}
Check 'Hash SHA256' ($src.Contains('sha256_hex') -and $schema.Contains('transaction_hash'))
Check 'Audit rapprochement' ($schema.Contains('bank_reconciliation_events'))
Check 'Positions séparées' ($src.Contains('bank_cash_positions') -and $src.Contains('imported_bank_cents') -and $src.Contains('theoretical_cents'))
Check 'Route UI Banque' ($ui.Contains('Page::Bank=>rsx!{BankManagementPage{refresh}}'))
Check 'Module exporté' ($lib.Contains('pub mod banking;'))
Check 'Pas de DROP/TRUNCATE' (-not ($schema -match '(?i)\b(DROP TABLE|TRUNCATE)\b'))
$failed=@($script:checks|Where-Object Status -eq 'FAIL')
$script:checks|ForEach-Object{"$($_.Status) | $($_.Name)"}
if($failed.Count){throw "S08_BANKING_GATE=FAIL ($($failed.Count))"}
'S08_BANKING_GATE=PASS'
