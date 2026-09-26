[CmdletBinding()]
param(
    [string]$ProjectPath = (Split-Path -Parent $PSScriptRoot),
    [switch]$SkipBackup,
    [switch]$SkipVerify
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

Set-Location -LiteralPath $ProjectPath

$snapshotDir = Join-Path $ProjectPath "snapshots"
$backupDir = Join-Path $ProjectPath "backups"

New-Item -ItemType Directory -Force -Path $snapshotDir | Out-Null
New-Item -ItemType Directory -Force -Path $backupDir | Out-Null

function Load-DotEnv {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw ".env introuvable : $Path"
    }

    foreach ($line in Get-Content -LiteralPath $Path -Encoding UTF8) {
        $trimmed = $line.Trim()

        if ($trimmed.Length -eq 0 -or $trimmed.StartsWith("#")) {
            continue
        }

        $eq = $trimmed.IndexOf("=")
        if ($eq -lt 1) {
            continue
        }

        $key = $trimmed.Substring(0, $eq).Trim()
        $value = $trimmed.Substring($eq + 1).Trim()

        if (($value.StartsWith('"') -and $value.EndsWith('"')) -or
            ($value.StartsWith("'") -and $value.EndsWith("'"))) {
            $value = $value.Substring(1, $value.Length - 2)
        }

        [Environment]::SetEnvironmentVariable($key, $value, "Process")
    }
}

function Require-Command {
    param([string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Commande introuvable : $Name"
    }
}

function Invoke-External {
    param(
        [string]$FilePath,
        [string[]]$ArgumentList
    )

    & $FilePath @ArgumentList
    if ($LASTEXITCODE -ne 0) {
        throw "Échec de commande : $FilePath $($ArgumentList -join ' ')"
    }
}

$envPath = Join-Path $ProjectPath ".env"
Load-DotEnv -Path $envPath

if ([string]::IsNullOrWhiteSpace($env:DATABASE_URL)) {
    throw "DATABASE_URL est absent du .env"
}

Require-Command "docker"
Require-Command "cargo"

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$backupPath = Join-Path $backupDir "pre-migrate-$timestamp.sql"
$reportPath = Join-Path $snapshotDir "migration-report-$timestamp.md"

$startedAt = Get-Date

if (-not $SkipVerify) {
    & (Join-Path $ProjectPath "scripts\verify-migrations.ps1")
    if ($LASTEXITCODE -ne 0) {
        throw "Le gate de migration a bloqué l'exécution."
    }
}

Write-Host ""
Write-Host "=== SCI FAMILY — SAFE MIGRATION ===" -ForegroundColor Cyan
Write-Host "Project : $ProjectPath"
Write-Host ""

Write-Host "[1/5] PostgreSQL..." -ForegroundColor Yellow
Invoke-External "docker" @("compose", "up", "-d", "--wait", "postgres")

Write-Host "[2/5] Health PostgreSQL..." -ForegroundColor Yellow
Invoke-External "docker" @("compose", "exec", "-T", "postgres", "pg_isready", "-U", "sci", "-d", "sci_family")

if (-not $SkipBackup) {
    Write-Host "[3/5] Backup..." -ForegroundColor Yellow

    $dumpArgs = @(
        "compose", "exec", "-T", "postgres",
        "pg_dump",
        "-U", "sci",
        "-d", "sci_family",
        "--no-owner",
        "--no-privileges"
    )

    & docker @dumpArgs | Out-File -LiteralPath $backupPath -Encoding utf8
    if ($LASTEXITCODE -ne 0) {
        throw "Échec du backup PostgreSQL."
    }

    if ((Get-Item -LiteralPath $backupPath).Length -eq 0) {
        throw "Le backup généré est vide : $backupPath"
    }

    $backupStatus = "CREATED: $backupPath"
} else {
    Write-Host "[3/5] Backup ignoré par option." -ForegroundColor DarkYellow
    $backupStatus = "SKIPPED"
}

Write-Host "[4/5] SQLx migration runner..." -ForegroundColor Yellow

$cargoArgs = @(
    "run",
    "--locked",
    "--features", "server",
    "--bin", "sci-family-migrate"
)

& cargo @cargoArgs 2>&1 | Tee-Object -Variable cargoOutput
$cargoExit = $LASTEXITCODE

if ($cargoExit -ne 0) {
    $joined = ($cargoOutput -join "`n")
    throw "Le migrateur Rust a échoué.`n$joined"
}

Write-Host "[5/5] Vérification de l'historique SQLx..." -ForegroundColor Yellow

$expectedCount = @(Get-ChildItem -LiteralPath (Join-Path $ProjectPath "migrations") -Filter "*.sql" -File).Count

$countQuery = "SELECT count(*) FROM _sqlx_migrations WHERE success = true;"
$appliedCountRaw = docker compose exec -T postgres psql -U sci -d sci_family -Atqc $countQuery
if ($LASTEXITCODE -ne 0) {
    throw "Impossible de lire _sqlx_migrations."
}

$appliedCount = [int]($appliedCountRaw | Select-Object -First 1)

if ($appliedCount -lt $expectedCount) {
    throw "Historique SQLx incomplet : $appliedCount appliquées / $expectedCount attendues."
}

$finishedAt = Get-Date
$duration = New-TimeSpan -Start $startedAt -End $finishedAt

$report = @"
# Migration report — $timestamp

Project: `D:\SCI\DEV\SCI-family-rust`

## Résultat

- Status: `SUCCESS`
- Started: $startedAt
- Finished: $finishedAt
- Duration: $($duration.ToString())
- Migration files: $expectedCount
- SQLx successful migrations: $appliedCount
- Backup: $backupStatus

## Commande Rust

`cargo run --locked --features server --bin sci-family-migrate`

## Règles appliquées

- Gate statique avant migration
- Backup avant migration (sauf `-SkipBackup`)
- Exécution via le même mécanisme SQLx que l'application
- Vérification de `_sqlx_migrations` après exécution

## Prochaine étape

`US-0103 — Fichiers d'état`
"@

Set-Content -LiteralPath $reportPath -Value $report -Encoding utf8

Write-Host ""
Write-Host "MIGRATION_STATUS=SUCCESS" -ForegroundColor Green
Write-Host "backup=$backupStatus"
Write-Host "report=$reportPath"
Write-Host ""
exit 0
