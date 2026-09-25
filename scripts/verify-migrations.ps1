[CmdletBinding()]
param(
    [string]$ProjectPath = (Split-Path -Parent $PSScriptRoot),
    [string]$MigrationPath = (Join-Path $ProjectPath "migrations")
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (-not (Test-Path -LiteralPath $MigrationPath -PathType Container)) {
    throw "Dossier migrations introuvable : $MigrationPath"
}

$migrations = @(Get-ChildItem -LiteralPath $MigrationPath -File -Filter "*.sql" | Sort-Object Name)

if ($migrations.Count -eq 0) {
    throw "Aucune migration SQL trouvée dans : $MigrationPath"
}

$allowedApprovedMarker = "-- SCI-DESTRUCTIVE: APPROVED"
$destructivePatterns = @(
    '(?is)\bDROP\s+TABLE\b',
    '(?is)\bDROP\s+COLUMN\b',
    '(?is)\bDROP\s+SCHEMA\b',
    '(?is)\bDROP\s+INDEX\b',
    '(?is)\bTRUNCATE(?:\s+TABLE)?\b',
    '(?is)\bDELETE\s+FROM\b'
)

$failures = New-Object System.Collections.Generic.List[string]
$seenVersions = @{}
$reportRows = New-Object System.Collections.Generic.List[object]

foreach ($migration in $migrations) {
    if ($migration.Name -notmatch '^(?<version>\d{4,})_(?<name>[A-Za-z0-9][A-Za-z0-9_-]*)\.sql$') {
        $failures.Add("Nom de migration invalide : $($migration.Name)")
        continue
    }

    $version = [int64]$Matches.version
    if ($seenVersions.ContainsKey($version)) {
        $failures.Add("Numéro de migration dupliqué : $version ($($seenVersions[$version]) / $($migration.Name))")
    } else {
        $seenVersions[$version] = $migration.Name
    }

    $raw = Get-Content -LiteralPath $migration.FullName -Raw -Encoding UTF8
    $withoutLineComments = [regex]::Replace($raw, '(?m)--.*$', '')
    $withoutBlockComments = [regex]::Replace($withoutLineComments, '(?s)/\*.*?\*/', '')
    $sqlForGate = $withoutBlockComments

    $approved = $raw -match [regex]::Escape($allowedApprovedMarker)
    $hits = New-Object System.Collections.Generic.List[string]

    foreach ($pattern in $destructivePatterns) {
        $match = [regex]::Match($sqlForGate, $pattern)
        if ($match.Success) {
            $hits.Add($match.Value.Trim())
        }
    }

    if ($hits.Count -gt 0 -and -not $approved) {
        $failures.Add("Opération(s) destructive(s) non approuvée(s) dans $($migration.Name) : $($hits -join ', ')")
    }

    $hash = (Get-FileHash -LiteralPath $migration.FullName -Algorithm SHA256).Hash

    $reportRows.Add([pscustomobject]@{
        Version = $version
        Name = $migration.Name
        Sha256 = $hash
        DestructiveDetected = ($hits.Count -gt 0)
        ApprovalMarker = $approved
        Status = if ($hits.Count -gt 0 -and -not $approved) { "FAIL" } else { "PASS" }
    })
}

$versions = @($seenVersions.Keys | Sort-Object)
for ($i = 1; $i -lt $versions.Count; $i++) {
    if ($versions[$i] -eq $versions[$i - 1]) {
        continue
    }
}

Write-Host ""
Write-Host "=== SCI FAMILY — VERIFY MIGRATIONS ===" -ForegroundColor Cyan
Write-Host "Project : $ProjectPath"
Write-Host "Folder  : $MigrationPath"
Write-Host "Files   : $($migrations.Count)"
Write-Host ""

$reportRows | Format-Table Version,Name,Status,DestructiveDetected,ApprovalMarker -AutoSize

if ($failures.Count -gt 0) {
    Write-Host ""
    Write-Host "MIGRATION_GATE=FAIL" -ForegroundColor Red
    foreach ($failure in $failures) {
        Write-Host " - $failure" -ForegroundColor Red
    }
    exit 1
}

Write-Host ""
Write-Host "MIGRATION_GATE=PASS" -ForegroundColor Green
Write-Host "migration_count=$($migrations.Count)"
Write-Host ""
exit 0
