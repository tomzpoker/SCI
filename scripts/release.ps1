[CmdletBinding()]
param([string]$Version='0.5.0',[string]$Channel='stable')
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
if(-not(Get-Command git -ErrorAction SilentlyContinue)){throw 'git introuvable.'}
foreach($gate in @('verify-s15-s16-final.ps1','verify-s17-s19-final.ps1','verify-s18-release.ps1','verify-s19-cycle.ps1')){
  & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot $gate)
  if($LASTEXITCODE -ne 0){throw "Gate en échec: $gate"}
}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'run-tests.ps1')
if($LASTEXITCODE -ne 0){throw 'Tests déterministes en échec.'}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'verify-git-hygiene.ps1')
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
$branch=(git branch --show-current).Trim();$commit=(git rev-parse HEAD).Trim();$date=(Get-Date -Format 'yyyy-MM-dd');$dirty=@(git status --porcelain)
if($dirty.Count -gt 0){throw 'Release refusée : working tree non propre après le contrôle des secrets.'}
$migs=(Get-ChildItem migrations -Filter '*.sql'|Sort-Object Name|ForEach-Object{$_.BaseName}) -join ', '
$report=@"
# Release $Version

- Date : $date
- Canal : $Channel
- Branche : $branch
- Commit : $commit
- Objectif : Release S18 après hardening S17 et validation système S19.
- Fonctionnalités : S01 → S19, tests/hardening, installation reproductible, upgrade, rollback, Git Sync, protection des secrets, cycle métier complet.
- Migrations : $migs
- Références : référentiels déjà versionnés du projet.
- Moteurs : règles, TVA, facturation, banque, trésorerie, fiscalité, documents, e-facturation, IA contrôlée.
- Tests : unitaires, intégration, workflows, régression, property-like, snapshots, imports, sécurité, temporalité, migrations, résilience, E2E.
- Breaking changes : aucune migration destructive introduite par S17 → S19.
- Compatibilité : Windows + PostgreSQL 17, Docker Compose, Rust toolchain déclarée.
- Rollback : backup de sécurité préalable puis restauration contrôlée selon `scripts/recovery.ps1`.
- Prochaine étape : maintenance et certification runtime continue.
"@
$path=Join-Path $Root "RELEASE_$Version.md";$report|Set-Content $path -Encoding UTF8
Write-Host "RELEASE_REPORT=$path"
Write-Host 'RELEASE_STATUS=PASS' -ForegroundColor Green
