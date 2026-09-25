[CmdletBinding()]
param(
    [string]$ProjectPath = (Split-Path -Parent $PSScriptRoot),
    [string]$TestStatus = "TO_VERIFY"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
Set-Location -LiteralPath $ProjectPath

function Get-DotEnvValue {
    param([string]$Name)

    $envFile = Join-Path $ProjectPath ".env"
    if (-not (Test-Path -LiteralPath $envFile -PathType Leaf)) {
        return $null
    }

    foreach ($line in Get-Content -LiteralPath $envFile -Encoding UTF8) {
        $trimmed = $line.Trim()
        if ($trimmed.Length -eq 0 -or $trimmed.StartsWith("#")) { continue }

        $prefix = "$Name="
        if ($trimmed.StartsWith($prefix)) {
            $value = $trimmed.Substring($prefix.Length).Trim()
            if (($value.StartsWith('"') -and $value.EndsWith('"')) -or
                ($value.StartsWith("'") -and $value.EndsWith("'"))) {
                $value = $value.Substring(1, $value.Length - 2)
            }
            return $value
        }
    }

    return $null
}

function Invoke-Captured {
    param(
        [string]$FilePath,
        [string[]]$ArgumentList
    )

    try {
        $output = & $FilePath @ArgumentList 2>&1
        [pscustomobject]@{
            Ok = ($LASTEXITCODE -eq 0)
            Text = ($output -join "`n").Trim()
        }
    } catch {
        [pscustomobject]@{
            Ok = $false
            Text = $_.Exception.Message
        }
    }
}

function Read-CargoVersion {
    $cargoToml = Join-Path $ProjectPath "Cargo.toml"
    if (-not (Test-Path -LiteralPath $cargoToml -PathType Leaf)) {
        return "UNKNOWN"
    }

    $content = Get-Content -LiteralPath $cargoToml -Raw -Encoding UTF8

    if ($content -match '(?m)^\s*version\s*=\s*"([^"]+)"') {
        return $Matches[1]
    }

    return "UNKNOWN"
}

function Get-GitState {
    $git = Get-Command git -ErrorAction SilentlyContinue
    if (-not $git -or -not (Test-Path -LiteralPath (Join-Path $ProjectPath ".git"))) {
        return @{
            Branch = "NOT_VERIFIED"
            Commit = "NOT_VERIFIED"
            Status = "NOT_VERIFIED"
            Remote = "NOT_VERIFIED"
        }
    }

    $branchResult = Invoke-Captured "git" @("branch", "--show-current")
    $commitResult = Invoke-Captured "git" @("rev-parse", "--short", "HEAD")
    $statusResult = Invoke-Captured "git" @("status", "--short", "--branch")
    $remoteResult = Invoke-Captured "git" @("remote", "get-url", "origin")

    return @{
        Branch = if ($branchResult.Ok -and $branchResult.Text) { $branchResult.Text } else { "UNKNOWN" }
        Commit = if ($commitResult.Ok -and $commitResult.Text) { $commitResult.Text } else { "UNKNOWN" }
        Status = if ($statusResult.Ok) {
            if ([string]::IsNullOrWhiteSpace($statusResult.Text)) { "CLEAN" } else { $statusResult.Text }
        } else { "UNKNOWN" }
        Remote = if ($remoteResult.Ok -and $remoteResult.Text) { $remoteResult.Text } else { "NOT_VERIFIED" }
    }
}

function Get-Migrations {
    $migrationPath = Join-Path $ProjectPath "migrations"
    if (-not (Test-Path -LiteralPath $migrationPath -PathType Container)) {
        return @()
    }

    return @(Get-ChildItem -LiteralPath $migrationPath -File -Filter "*.sql" |
        Sort-Object Name |
        ForEach-Object {
            [pscustomobject]@{
                Name = $_.Name
                Hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
            }
        })
}

function Get-Engines {
    $manifest = Join-Path $ProjectPath "engines\manifest.toml"
    if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) {
        return "NOT_VERIFIED — engines/manifest.toml absent"
    }

    $lines = @(Get-Content -LiteralPath $manifest -Encoding UTF8 |
        Where-Object { $_ -match '^\s*(\[[^\]]+\]|version\s*=|platform\s*=|sha256\s*=)' })

    if ($lines.Count -eq 0) {
        return "TO_VERIFY — manifest present but no version/platform/SHA entries detected"
    }

    return ($lines -join "`n")
}

function Get-DatabaseState {
    $url = Get-DotEnvValue -Name "DATABASE_URL"
    if ([string]::IsNullOrWhiteSpace($url)) {
        return @{
            Status = "NOT_VERIFIED — DATABASE_URL absent"
            Applied = "TO_VERIFY"
            Pending = "TO_VERIFY"
        }
    }

    $docker = Get-Command docker -ErrorAction SilentlyContinue
    if (-not $docker) {
        return @{
            Status = "NOT_VERIFIED — docker unavailable"
            Applied = "TO_VERIFY"
            Pending = "TO_VERIFY"
        }
    }

    $health = Invoke-Captured "docker" @("compose", "exec", "-T", "postgres", "pg_isready", "-U", "sci", "-d", "sci_family")
    if (-not $health.Ok) {
        return @{
            Status = "NOT_VERIFIED — PostgreSQL not ready"
            Applied = "TO_VERIFY"
            Pending = "TO_VERIFY"
        }
    }

    $countRaw = & docker compose exec -T postgres psql -U sci -d sci_family -Atqc "SELECT count(*) FROM _sqlx_migrations WHERE success = true;" 2>$null
    if ($LASTEXITCODE -ne 0) {
        return @{
            Status = "CONNECTED — _sqlx_migrations unreadable"
            Applied = "TO_VERIFY"
            Pending = "TO_VERIFY"
        }
    }

    $applied = [int]($countRaw | Select-Object -First 1)
    $migrationFiles = @(Get-Migrations)
    $available = $migrationFiles.Count

    return @{
        Status = "READY"
        Applied = [string]$applied
        Pending = [string][Math]::Max(0, $available - $applied)
    }
}

$version = Read-CargoVersion
$git = Get-GitState
$migrations = @(Get-Migrations)
$engines = Get-Engines
$db = Get-DatabaseState
$now = Get-Date -Format "yyyy-MM-dd HH:mm:ss zzz"
$latestMigration = if ($migrations.Count -gt 0) { $migrations[-1].Name } else { "NONE" }
$latestMigrationNumber = if ($migrations.Count -gt 0) { $migrations[-1].Name.Substring(0,4) } else { "0000" }
$currentPhase = "Étape 1 — socle multi-sociétés + profils d’exploitation"
$currentStory = "S19 — Cycle métier consolidé / Release $version"
$nextObjective = "Certification runtime Windows/MSVC + Docker/PostgreSQL"

$projectState = @"
# PROJECT_STATE — SCI Family Rust

> Généré par `scripts\update-state.ps1`.

- Generated: $now
- Projet: SCI Family Rust / SCI Family Pilot
- Chemin: `$ProjectPath`
- Version: `$version`
- Phase: `$currentPhase`
- Story courante: `$currentStory`
- Prochain objectif: `$nextObjective`

## Fonctionnalités

- SCI / onboarding: PRESENT
- Associés: PRESENT
- Biens / lots: PRESENT
- Locataires / baux: PRESENT
- Facturation / paiements: PRESENT
- TVA / échéances: PRESENT
- Banque / import CSV / rapprochement: PRESENT
- Automatisation / tâches / audit: PRESENT
- Documents: PRESENT
- Intelligence documentaire: PRESENT
- Prévision de trésorerie: PRESENT

## Tests

- Test status transmis: `$TestStatus`
- Build dynamique: TO_VERIFY si non fourni séparément
- E2E: TO_VERIFY

## Git

- Branch: $($git.Branch)
- Commit: $($git.Commit)
- Working tree: $($git.Status)
- Remote: $($git.Remote)

## Problèmes / dettes connus

- Validation Windows réelle encore requise lorsque non exécutée.
- Multi-sociétés: IMPLEMENTED pour les profils opérationnels de l’étape 1 ; le cloisonnement par entité est contrôlé par rôle.
- UX mobile/PWA/accessibilité exhaustive: TO_VERIFY.

## Prochain objectif

`$nextObjective`
"@

$architectureState = @"
# ARCHITECTURE_STATE — SCI Family Rust

> Généré par `scripts\update-state.ps1`.

- Generated: $now
- Rust / edition: 2024
- UI: Dioxus 0.7
- DB: PostgreSQL 17-alpine
- Migration engine: SQLx
- Windows target: MSVC
- WASM target: wasm32-unknown-unknown

## Flux

`Dioxus UI → Server functions/services → domain/infrastructure → SQLx → PostgreSQL`

## Migrations

$($migrations | ForEach-Object { "- $($_.Name) — SHA256 `$($_.Hash)`" } | Out-String)

## Modules métier

SCI, associés, biens, lots, locataires, baux, facturation, paiements, banque, TVA, échéances, automatisation, tâches, audit, documents, intelligence/OCR/LLM, trésorerie.

## Principes

- préserver le vertical slice existant ;
- préférer les évolutions additives ;
- ne pas réintroduire Supabase comme runtime ;
- ne jamais laisser l'IA écrire arbitrairement en base ;
- conserver la validation métier et les post-conditions ;
- protéger les migrations destructives par le gate prévu.

## État

Compilation et certification E2E: TO_VERIFY
"@

$roadmapState = @"
# ROADMAP_STATE — SCI Family Rust

> Généré par `scripts\update-state.ps1`.

- Generated: $now
- Sprint courant: `S19 — Cycle métier consolidé`

## État des stories

| Story | État |
|---|---|
| S01 → S16 — socle métier | IMPLEMENTED |
| S17 — tests / hardening | IMPLEMENTED |
| S18 — release / recovery | IMPLEMENTED |
| S19 — cycle métier | IMPLEMENTED |
| Étape 1 — multi-sociétés / profils d’exploitation | IMPLEMENTED / À CERTIFIER RUNTIME |

## Règle

Chaque story suit `Research → Design → Plan → Execute → Review → Ship`.

## Prochain objectif exact

`$nextObjective`
"@

$databaseState = @"
# DATABASE_STATE — SCI Family Rust

> Généré par `scripts\update-state.ps1`.

- Generated: $now
- PostgreSQL: 17-alpine
- Database: sci_family
- Host port: 55432
- SQLx history: `_sqlx_migrations`

## Migrations disponibles

$($migrations | ForEach-Object { "- $($_.Name)" } | Out-String)

Nombre disponible: $($migrations.Count)
Nombre appliqué avec succès: $($db.Applied)
Nombre potentiellement en attente: $($db.Pending)
Connexion DB: $($db.Status)

## Modèle

workspaces, scis, associates, properties, units, tenants, leases, invoices, payments, bank_transactions, automation_rules, tasks, tax_deadlines, documents, forecast_snapshots, audit_events, app_settings.

## Sécurité

Aucune migration n'est exécutée par ce script.
Les backups et l'exécution des migrations restent du ressort de `scripts\migrate.ps1`.
"@

$versionState = @"
# VERSION_STATE — SCI Family Rust

> Généré par `scripts\update-state.ps1`.

- Generated: $now
- Package: `sci-family-pilot`
- Project version: `$version`
- Sprint: `S19`
- Story: `S19 — Cycle métier consolidé / Release $version`

## Versions techniques

- Dioxus: `0.7`
- PostgreSQL: `17-alpine`
- Rust toolchain normative: `1.98.1-x86_64-pc-windows-msvc`
- WASM: `wasm32-unknown-unknown`

## Version indépendante

- Schema version: migration `$latestMigration` / version `$latestMigrationNumber`
- Reference-data version: TO_VERIFY
- Rules version: TO_VERIFY
- Engine versions: voir manifeste ci-dessous

## Moteurs

```text
$engines
```

## Git

- Branch: $($git.Branch)
- Commit: $($git.Commit)
- Remote: $($git.Remote)

## Release

Chaque release devra conserver un rapport dans `versions/` avec version, date, branche, commit, objectif, fonctionnalités, migrations, reference data, moteurs, tests, breaking changes, compatibilité, rollback et prochaine étape.

## Prochaine étape

`$nextObjective`
"@

Set-Content -LiteralPath (Join-Path $ProjectPath "PROJECT_STATE.md") -Value $projectState -Encoding utf8
Set-Content -LiteralPath (Join-Path $ProjectPath "ARCHITECTURE_STATE.md") -Value $architectureState -Encoding utf8
Set-Content -LiteralPath (Join-Path $ProjectPath "ROADMAP_STATE.md") -Value $roadmapState -Encoding utf8
Set-Content -LiteralPath (Join-Path $ProjectPath "DATABASE_STATE.md") -Value $databaseState -Encoding utf8
Set-Content -LiteralPath (Join-Path $ProjectPath "VERSION_STATE.md") -Value $versionState -Encoding utf8

Write-Host ""
Write-Host "=== SCI FAMILY — STATE UPDATE ===" -ForegroundColor Cyan
Write-Host "Version   : $version"
Write-Host "Branch    : $($git.Branch)"
Write-Host "Commit    : $($git.Commit)"
Write-Host "Migrations: $($migrations.Count) available / $($db.Applied) applied"
Write-Host "Database  : $($db.Status)"
Write-Host "Test      : $TestStatus"
Write-Host ""
Write-Host "STATE_UPDATE=PASS" -ForegroundColor Green
Write-Host "Files written:"
Write-Host " - PROJECT_STATE.md"
Write-Host " - ARCHITECTURE_STATE.md"
Write-Host " - ROADMAP_STATE.md"
Write-Host " - DATABASE_STATE.md"
Write-Host " - VERSION_STATE.md"
Write-Host ""
exit 0
