[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
foreach($p in @('fixtures/e2e/locative_cycle.json','tests/e2e_cycle.rs','tests/integration/modules_12.rs','S19_SYSTEM_CERTIFICATION.md','DEVELOPER_HANDOFF.md')){if(-not(Test-Path $p)){throw "Artefact S19 absent: $p"}}
$cycle=Get-Content fixtures/e2e/locative_cycle.json -Raw | ConvertFrom-Json
if($cycle.steps.Count -ne 15){throw 'Cycle locatif incomplet'}
$expected='SCI','Immeuble','Lot','Locataire','Bail','Loyer','Facture','Encaissement','Banque','Rapprochement','TVA','Fiscalité','Calendrier','Document','Archive'
for($i=0;$i -lt $expected.Count;$i++){if($cycle.steps[$i] -ne $expected[$i]){throw "Étape S19 incorrecte: $($cycle.steps[$i])"}}
$test=Get-Content tests/integration/modules_12.rs -Raw
if($test -notmatch 'archive_document_record'){throw 'Archive absente du cycle runtime'}
Write-Host 'S19_CYCLE_STATIC_GATE=PASS' -ForegroundColor Green
