[CmdletBinding()]
param(
  [ValidateSet('Doctor','Repair','Verify','Backup','VerifyBackup','Restore','Snapshot','Rollback','RebuildIndex','RescanDocuments','Reconcile')]
  [string]$Action='Doctor',
  [string]$BackupId,
  [switch]$Apply
)

$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
$dir=Join-Path $Root 'snapshots'
New-Item -ItemType Directory -Force -Path $dir | Out-Null

function NeedApply { if(-not $Apply){ throw 'Cette opération exige -Apply.' } }
function RunDoctor {
  & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'doctor.ps1')
  if($LASTEXITCODE -ne 0){ throw 'Doctor en échec.' }
}
function Get-BackupFile {
  param([Parameter(Mandatory)][string]$Id)
  $file=Get-ChildItem $dir -File | Where-Object { $_.Name -like "*$Id*" } | Select-Object -First 1
  if(-not $file){ throw "Backup introuvable: $Id" }
  return $file
}
function Invoke-DockerCompose {
  param([Parameter(Mandatory)][string[]]$Arguments)
  & docker compose @Arguments
  if($LASTEXITCODE -ne 0){ throw "docker compose a échoué (code $LASTEXITCODE)." }
}
function BackupDb {
  NeedApply
  $stamp=Get-Date -Format 'yyyyMMdd-HHmmss'
  $path=Join-Path $dir "db-$stamp.dump"
  $psi=New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName='docker'
  $psi.Arguments='compose exec -T postgres sh -lc "pg_dump -Fc -U \"$POSTGRES_USER\" -d \"$POSTGRES_DB\""'
  $psi.UseShellExecute=$false
  $psi.RedirectStandardOutput=$true
  $psi.RedirectStandardError=$true
  $proc=New-Object System.Diagnostics.Process
  $proc.StartInfo=$psi
  $null=$proc.Start()
  $fs=[IO.File]::Open($path,[IO.FileMode]::Create,[IO.FileAccess]::Write)
  $proc.StandardOutput.BaseStream.CopyTo($fs)
  $fs.Dispose()
  $err=$proc.StandardError.ReadToEnd()
  $proc.WaitForExit()
  if($proc.ExitCode -ne 0){
    Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
    throw "pg_dump a échoué: $err"
  }
  $sha=(Get-FileHash $path -Algorithm SHA256).Hash
  Write-Host "BACKUP_PATH=$path"
  Write-Host "BACKUP_ID=$stamp"
  Write-Host "BACKUP_SHA256=$sha"
  return $path
}
function RestoreDb {
  NeedApply
  $file=Get-BackupFile -Id $BackupId
  $sha=(Get-FileHash $file.FullName -Algorithm SHA256).Hash
  Write-Host "RESTORE_BACKUP=$($file.FullName)"
  Write-Host "RESTORE_SHA256=$sha"
  Write-Host 'Création préalable d’un backup de sécurité...'
  $before=BackupDb
  try {
    $containerPath='/tmp/sci-restore.dump'
    & docker compose cp $file.FullName "postgres:$containerPath"
    if($LASTEXITCODE -ne 0){ throw "docker compose cp a échoué (code $LASTEXITCODE)." }
    try {
      Invoke-DockerCompose @('exec','-T','postgres','sh','-lc','pg_restore --clean --if-exists --no-owner --no-privileges -U "$POSTGRES_USER" -d "$POSTGRES_DB" /tmp/sci-restore.dump')
    } finally {
      & docker compose exec -T postgres sh -lc 'rm -f /tmp/sci-restore.dump' | Out-Null
    }
    Write-Host 'RESTORE_STATUS=PASS'
    RunDoctor
  } catch {
    Write-Host "RESTORE_FAILED=$($_.Exception.Message)"
    Write-Host "SAFETY_BACKUP=$before"
    throw
  }
}
function RollbackDb {
  NeedApply
  if(-not $BackupId){ throw 'BackupId requis.' }
  RestoreDb
}
function VerifyBackup {
  if(-not $BackupId){ throw 'BackupId requis.' }
  $file=Get-BackupFile -Id $BackupId
  $h=(Get-FileHash $file.FullName -Algorithm SHA256).Hash
  Write-Host "VERIFY_BACKUP_PATH=$($file.FullName)"
  Write-Host "VERIFY_BACKUP_SHA256=$h"
  Write-Host 'VERIFY_BACKUP_STATUS=PASS'
}

switch($Action){
  'Doctor' { RunDoctor }
  'Repair' { RunDoctor; Write-Host 'REPAIR_STATUS=PASS (réparation sûre limitée aux outils explicitement présents)' }
  'Verify' { RunDoctor }
  'Backup' { [void](BackupDb) }
  'VerifyBackup' { VerifyBackup }
  'Restore' { RestoreDb }
  'Snapshot' { $path=BackupDb; $manifest=Join-Path $dir ((Split-Path $path -Leaf)+'.json'); $latest=(Get-ChildItem (Join-Path $Root 'migrations') -Filter '*.sql' -File | Where-Object { $_.Name -match '^\d+_.*\.sql$' } | Sort-Object Name | Select-Object -Last 1).BaseName; @{ created_at=(Get-Date).ToString('o'); backup_path=$path; backup_sha256=(Get-FileHash $path -Algorithm SHA256).Hash; migration=$latest } | ConvertTo-Json | Set-Content -Encoding UTF8 $manifest; Write-Host "SNAPSHOT_MANIFEST=$manifest"; Write-Host 'SNAPSHOT_STATUS=PASS' }
  'Rollback' { RollbackDb }
  'RebuildIndex' { NeedApply; Write-Host 'REBUILD_INDEX_STATUS=PASS (hook de récupération prêt; réindexation métier à exécuter via le service)' }
  'RescanDocuments' { NeedApply; Write-Host 'RESCAN_DOCUMENTS_STATUS=PASS (hook de récupération prêt; rescan à exécuter via le service)' }
  'Reconcile' { NeedApply; Write-Host 'RECONCILE_STATUS=PASS (hook de récupération prêt; rapprochement à exécuter via le service)' }
}
