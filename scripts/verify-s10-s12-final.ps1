$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$checks = @(
  (Test-Path "$root\migrations\0020_fiscality.sql"),
  (Test-Path "$root\migrations\0021_unpaid_recovery.sql"),
  (Test-Path "$root\migrations\0022_document_generation.sql"),
  ((Get-Content "$root\migrations\0020_fiscality.sql" -Raw) -match 'vat_advances'),
  ((Get-Content "$root\src\fiscal.rs" -Raw) -match 'register_vat_advance'),
  ((Get-Content "$root\src\collections.rs" -Raw) -match 'UNDERPAYMENT'),
  ((Get-Content "$root\src\generation\mod.rs" -Raw) -match 'double precision'),
  ((Get-Content "$root\src\generation\pdf.rs" -Raw) -match '/Type /Page'),
  ((Get-Content "$root\src\lib.rs" -Raw) -match 'pub mod fiscal'),
  ((Get-Content "$root\src\lib.rs" -Raw) -match 'pub mod collections'),
  ((Get-Content "$root\src\lib.rs" -Raw) -match 'pub mod generation')
)
if($checks -contains $false){ throw "S10-S12 static gate FAILED" }
Write-Host "S10_S12_STATIC_GATE=PASS"
