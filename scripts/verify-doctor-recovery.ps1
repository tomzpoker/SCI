[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot

$required = @(
    'scripts/doctor.ps1',
    'scripts/recovery.ps1',
    'scripts/check-project.ps1',
    'S01_US-0105_README.md',
    'versions/S01_US-0105.md',
    'PROJECT_STATE.md',
    'ARCHITECTURE_STATE.md',
    'ROADMAP_STATE.md',
    'DATABASE_STATE.md',
    'VERSION_STATE.md'
)

$failures = New-Object System.Collections.Generic.List[string]
foreach ($rel in $required) {
    if (-not (Test-Path (Join-Path $Root $rel) -PathType Leaf)) {
        [void]$failures.Add("missing:$rel")
    }
}

$scanFiles = @(
    (Join-Path $Root 'scripts/doctor.ps1'),
    (Join-Path $Root 'scripts/recovery.ps1'),
    (Join-Path $Root 'scripts/check-project.ps1')
)

foreach ($file in $scanFiles) {
    if (-not (Test-Path $file)) { continue }
    $text = Get-Content $file -Raw
    if ($text -match '(?i)msys2|\\msys64|ucrt64|mingw|gnu') { [void]$failures.Add("forbidden-gnu-reference:$([IO.Path]::GetFileName($file))") }
    if ($text -match '(?i)docker\s+compose\s+down\s+-v|docker\s+volume\s+rm|git\s+reset\s+--hard|git\s+clean\s+-f|DROP\s+(TABLE|SCHEMA|COLUMN|INDEX)|TRUNCATE|DELETE\s+FROM') {
        [void]$failures.Add("unsafe-recovery-pattern:$([IO.Path]::GetFileName($file))")
    }
}

$doctor = Get-Content (Join-Path $Root 'scripts/doctor.ps1') -Raw
$recovery = Get-Content (Join-Path $Root 'scripts/recovery.ps1') -Raw
foreach ($token in @('PROJECT_STATE.md','ARCHITECTURE_STATE.md','ROADMAP_STATE.md','DATABASE_STATE.md','VERSION_STATE.md','engines/manifest.toml','storage_locations','_sqlx_migrations')) {
    if (($doctor + $recovery) -notmatch [regex]::Escape($token)) {
        [void]$failures.Add("coverage-missing:$token")
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    Write-Host 'DOCTOR_RECOVERY_STATIC_GATE=FAIL'
    exit 1
}

Write-Host 'DOCTOR_RECOVERY_STATIC_GATE=PASS'
Write-Host 'Checks: scripts, state files, GNU/MSYS2 absence, destructive recovery guards, DB/migration/document/engine/Git coverage.'
exit 0
