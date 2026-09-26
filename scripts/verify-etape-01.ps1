$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$m=Get-ChildItem (Join-Path $root 'migrations') -Filter '*.sql' | Sort-Object Name
$last=$m[-1].Name
if($last -ne '0029_multi_company_operating_profiles.sql'){throw "Dernière migration inattendue: $last"}
$sql=Get-Content (Join-Path $root 'migrations\0029_multi_company_operating_profiles.sql') -Raw
foreach($needle in @('SARL_IS_GARAGE_SANS_SAV','SCI_LOCATIVE_IR','vat_basis IN (''COLLECTION'',''DEBIT'')','vehicle_stock','parts_stock','consignment','workshop')){if($sql -notlike "*$needle*"){throw "Marqueur absent: $needle"}}
$lib=Get-Content (Join-Path $root 'src\lib.rs') -Raw;if($lib -notmatch 'pub mod business_profiles;'){throw 'Module business_profiles absent'}
$bp=Get-Content (Join-Path $root 'src\business_profiles.rs') -Raw;foreach($needle in @('list_business_profile_catalog','set_entity_business_profile','current_entity_business_profile')){if($bp -notlike "*$needle*"){throw "Fonction absente: $needle"}}
Write-Host 'ETAPE_01_STATIC_GATE=PASS'
Write-Host "LATEST_MIGRATION=$last"
