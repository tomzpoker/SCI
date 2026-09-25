[CmdletBinding()]
param([Parameter(Position=0)][ValidateSet('doctor','verify','repair','backup','verify-backup','restore','snapshot','rollback','rebuild-index','rescan-documents','reconcile')][string]$Command='doctor',[string]$BackupId,[switch]$Apply)
$map=@{doctor='Doctor';verify='Verify';repair='Repair';backup='Backup';'verify-backup'='VerifyBackup';restore='Restore';snapshot='Snapshot';rollback='Rollback';'rebuild-index'='RebuildIndex';'rescan-documents'='RescanDocuments';reconcile='Reconcile'}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'recovery.ps1') -Action $map[$Command] -BackupId $BackupId -Apply:$Apply;exit $LASTEXITCODE
