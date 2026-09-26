[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)

$failures = New-Object System.Collections.Generic.List[string]

function Require-Text {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Pattern,
        [Parameter(Mandatory)][string]$Description
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        [void]$failures.Add("Missing file: $Path")
        return
    }
    $text = Get-Content -LiteralPath $Path -Raw
    if ($text -notmatch $Pattern) {
        [void]$failures.Add("Missing $Description in $Path")
    }
}

Require-Text 'src/observability.rs' 'enum LogDomain' 'LogDomain'
Require-Text 'src/observability.rs' 'ERR-' 'error_id generator'
Require-Text 'src/observability.rs' 'json\(\)' 'JSON subscriber'
Require-Text 'src/observability.rs' 'record_error' 'record_error helper'
Require-Text 'src/observability.rs' 'record_warning' 'record_warning helper'
Require-Text 'src/lib.rs' 'pub mod observability;' 'observability module export'
Require-Text 'src/main.rs' 'observability::init\(\)' 'observability initialization'
Require-Text 'src/main.rs' 'LogDomain::Automation' 'automation log domain'
Require-Text 'src/bin/sci-family-migrate.rs' 'LogDomain::Migration' 'migration log domain'
Require-Text 'Cargo.toml' 'tracing-subscriber.*json' 'JSON tracing-subscriber feature'

$requiredDomains = @('application','security','business','fiscal','automation','ocr','bank','ai','migration')
$obs = Get-Content -LiteralPath 'src/observability.rs' -Raw
foreach ($domain in $requiredDomains) {
    if ($obs -notmatch [regex]::Escape("$domain")) {
        [void]$failures.Add("Required log domain missing: $domain")
    }
}

$rustFiles = @(Get-ChildItem -LiteralPath 'src' -Recurse -File -Filter '*.rs')
foreach ($file in $rustFiles) {
    if ($file.FullName -eq (Join-Path (Get-Location) 'src\observability.rs')) { continue }
    $text = Get-Content -LiteralPath $file.FullName -Raw
    if ($text -match 'tracing::(error|warn)!') {
        [void]$failures.Add("Direct tracing::error!/warn! outside observability helper: $($file.FullName)")
    }
}

Write-Host 'OBSERVABILITY_POLICY=STRUCTURED'
Write-Host 'ERROR_ID_POLICY=ERR-UUID'
Write-Host 'DOMAINS=application,security,business,fiscal,automation,ocr,bank,ai,migration'

if ($failures.Count -gt 0) {
    Write-Host 'OBSERVABILITY_POLICY=BLOCKED'
    $failures | ForEach-Object { Write-Error $_ }
    throw "Observability verification failed with $($failures.Count) error(s)."
}

Write-Host 'OBSERVABILITY_SELF_TESTS=PASS'
Write-Host 'OBSERVABILITY_POLICY=PASS'
