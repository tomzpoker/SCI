[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
foreach($p in @('tests/pure_hardening.rs','tests/snapshot_documents.rs','tests/fiscal_fixtures.rs','tests/workflow_s17.rs','tests/resilience.rs','tests/migration_manifest.rs','tests/security_hardening.rs','tests/regression.rs','tests/temporal_boundaries.rs','tests/imports.rs','tests/security.rs','tests/migrations_runtime.rs','TEST_MATRIX_S17.md','scripts/run-tests.ps1','fixtures/fiscal/vat_basic.json')){if(-not(Test-Path $p)){throw "Artefact absent: $p"}}
$migs=Get-ChildItem migrations -Filter '*.sql'|Sort-Object Name;$nums=@($migs|ForEach-Object{[int]([regex]::Match($_.Name,'^\d+').Value)});$expected=1..$nums.Count;if(($nums -join ',')-ne($expected -join ',')){throw 'Migrations non contiguës'}
$json=Get-Content fixtures/fiscal/vat_basic.json -Raw|ConvertFrom-Json;if(-not $json.rule_version -or -not $json.source.reference){throw 'Fixture fiscale incomplète'}
Write-Host 'S17_STATIC_GATE=PASS' -ForegroundColor Green
