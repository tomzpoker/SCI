$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
$m=Get-ChildItem (Join-Path $Root 'migrations') -Filter '*.sql' | Where-Object {$_.Name -match '^\d+_.*\.sql$'} | Sort-Object Name
$v=@($m | ForEach-Object {[int]([regex]::Match($_.Name,'^\d+').Value)})
if(($v -join ',') -ne ((1..27)-join ',')){throw 'Migrations attendues 0001..0027'}
$sql=Get-Content (Join-Path $Root 'migrations/0027_ux_zero_saisie.sql') -Raw
if($sql -match '(?i)DROP\s+TABLE|TRUNCATE\s+|DROP\s+COLUMN'){throw 'SQL destructive dans 0027'}
foreach($x in 'SCI','REGIME_FISCAL','TVA','BIENS','LOTS','LOCATAIRES','BAUX','BANQUE','DOCUMENTS','ASSOCIES','IMPORTS','AUTOMATISATIONS','NOTIFICATIONS','IA'){if($sql -notmatch [regex]::Escape("'$x'")){throw "Onboarding absent: $x"}}
foreach($f in 'src/ux.rs','assets/main.css','public/main.css','migrations/0027_ux_zero_saisie.sql'){if(-not(Test-Path(Join-Path $Root $f))){throw "Fichier absent: $f"}}
Write-Host 'S15_GATE=PASS'
Write-Host 'MIGRATIONS=0001..0027'
Write-Host 'DESTRUCTIVE_S15=NONE'
