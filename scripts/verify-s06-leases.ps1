$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
$required=@('migrations\0016_leases_locations.sql','src\leases.rs','src\domain.rs','src\ui.rs','src\lib.rs')
foreach($f in $required){if(!(Test-Path (Join-Path $root $f))){throw "Missing $f"}}
$sql=Get-Content (Join-Path $root 'migrations\0016_leases_locations.sql') -Raw
foreach($x in @('lease_clauses','lease_index_values','lease_rent_revisions','lease_charge_rules','lease_rent_reductions','lease_deposits','lease_guarantees','lease_entry_fees','lease_contract_events','signature_date','destination','rent_frequency','vat_mode','index_cap_bp')){if($sql -notmatch [regex]::Escape($x)){throw "Missing SQL marker $x"}}
$src=Get-Content (Join-Path $root 'src\leases.rs') -Raw
foreach($x in @('create_structured_lease','save_lease_clause','save_lease_index','calculate_lease_rent_revision','add_lease_charge','add_lease_reduction','add_lease_deposit','add_lease_guarantee','set_lease_entry_fee','LeasesPage','calculate_indexed_rent')){if($src -notmatch [regex]::Escape($x)){throw "Missing code marker $x"}}
$bad=$sql -split "`n" | Where-Object {$_ -notmatch '^\s*--' -and $_ -match '(?i)\b(DROP\s+(TABLE|COLUMN|SCHEMA|INDEX)|TRUNCATE|DELETE\s+FROM)\b'}
if($bad){throw ('Destructive SQL detected: '+($bad -join ' | '))}
Write-Host 'S06_LEASES_GATE=PASS'
