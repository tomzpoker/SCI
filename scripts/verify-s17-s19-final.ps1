[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
function Need([string]$Path){if(-not(Test-Path $Path)){throw "Fichier requis absent: $Path"}}
foreach($p in @('tests/pure_hardening.rs','tests/snapshot_documents.rs','tests/fiscal_fixtures.rs','tests/workflow_s17.rs','tests/resilience.rs','tests/migration_manifest.rs','tests/security_hardening.rs','tests/e2e_cycle.rs','scripts/migrate.ps1','scripts/run-tests.ps1','scripts/release.ps1','scripts/git-sync.ps1','scripts/verify-git-hygiene.ps1','fixtures/e2e/locative_cycle.json','fixtures/snapshots/letter_template.txt','fixtures/snapshots/letter_expected.txt')){Need $p}
$migs=Get-ChildItem migrations -Filter '*.sql'|Sort-Object Name;$nums=@($migs|ForEach-Object{[int]([regex]::Match($_.Name,'^\d+').Value)});$expected=1..$nums.Count;if(($nums -join ',')-ne($expected -join ',')){throw 'Migrations non contiguës'}
$all=Get-ChildItem tests -Recurse -File|ForEach-Object{Get-Content $_.FullName -Raw};if(-not(($all -join "`n") -match '31/12|12/31|23:59')){throw 'Temporalité 31/12 absente'}
if(-not(($all -join "`n") -match 'OCR_UNAVAILABLE|OCR_FAILED')){throw 'Résilience OCR absente'}
if(-not((Get-Content .gitignore -Raw) -match 'snapshots/' -and (Get-Content .gitignore -Raw) -match '\*\.dump')){throw '.gitignore insuffisant'}
Write-Host "MIGRATIONS=$($migs.Count)"
Write-Host "TEST_FILES=$((Get-ChildItem tests -Recurse -File).Count)"
Write-Host 'S17_S19_STATIC_GATE=PASS' -ForegroundColor Green
