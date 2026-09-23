$ErrorActionPreference = 'Continue'
Set-Location (Split-Path -Parent $PSScriptRoot)
$env:Path = "C:\msys64\ucrt64\bin;$env:USERPROFILE\.cargo\bin;$env:Path"

$checks = @()
$checks += [pscustomobject]@{Name='Rust'; OK=[bool](Get-Command rustc -ErrorAction SilentlyContinue)}
$checks += [pscustomobject]@{Name='Cargo'; OK=[bool](Get-Command cargo -ErrorAction SilentlyContinue)}
$checks += [pscustomobject]@{Name='Git'; OK=[bool](Get-Command git -ErrorAction SilentlyContinue)}
$checks += [pscustomobject]@{Name='Migration dir'; OK=(Test-Path 'migrations')}
$checks += [pscustomobject]@{Name='Engine manifest'; OK=(Test-Path 'engines/manifest.toml')}
$checks += [pscustomobject]@{Name='Project state'; OK=(Test-Path 'PROJECT_STATE.md')}
$checks += [pscustomobject]@{Name='Database state'; OK=(Test-Path 'DATABASE_STATE.md')}
$checks += [pscustomobject]@{Name='Roadmap state'; OK=(Test-Path 'ROADMAP_STATE.md')}
$checks | Format-Table -AutoSize
if ($checks.Where({-not $_.OK}).Count -gt 0) { exit 1 }
Write-Host 'STATUS ............... READY' -ForegroundColor Green
