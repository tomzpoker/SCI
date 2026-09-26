[CmdletBinding()]
param([switch]$SkipDatabase)
$ErrorActionPreference='Stop';$Root=Split-Path -Parent $PSScriptRoot;$ReportDir=Join-Path $Root 'snapshots';New-Item -ItemType Directory -Force -Path $ReportDir|Out-Null;$ReportPath=Join-Path $ReportDir ('doctor-'+(Get-Date -Format 'yyyyMMdd-HHmmss')+'.md');$fail=0;$warn=0;$report=New-Object System.Collections.Generic.List[string]
function Add-Check([string]$Name,[string]$Status,[string]$Detail=''){if($Status -eq 'FAIL'){$script:fail++}elseif($Status -eq 'WARN'){$script:warn++};$line="- **$Name** : $Status";if($Detail){$line+=" — $Detail"};$report.Add($line)|Out-Null;Write-Host $line}
function Has([string]$Path){Test-Path (Join-Path $Root $Path)}
Add-Check 'Application files' (if((Has 'Cargo.toml') -and (Has 'src') -and (Has 'migrations')){'PASS'}else{'FAIL'})
Add-Check 'Rules' (if(Has 'src/rules.rs'){'PASS'}else{'FAIL'})
Add-Check 'References' (if(Has 'src/reproducibility.rs'){'PASS'}else{'FAIL'})
Add-Check 'Documents/OCR' (if((Has 'src/documents/ocr.rs') -and (Has 'src/documents/workflow.rs')){'PASS'}else{'FAIL'})
Add-Check 'Bank' (if(Has 'src/banking.rs'){'PASS'}else{'FAIL'})
Add-Check 'Automation' (if(Has 'src/workflow.rs'){'PASS'}else{'FAIL'})
Add-Check 'IA' (if(Has 'src/assistant') -and (Has 'src/assistant/tools.rs'){'PASS'}else{'FAIL'})
Add-Check 'Security' (if(Has 'src/security.rs') -and (Has 'migrations/0028_security_recovery.sql'){'PASS'}else{'FAIL'})
Add-Check 'Tests' (if(Has 'tests'){'PASS'}else{'WARN'})
$files=Get-ChildItem (Join-Path $Root 'migrations') -Filter '*.sql'|Where-Object{$_.Name -match '^\d+_.*\.sql$'}|Sort-Object Name;$vers=@($files|ForEach-Object{[int]([regex]::Match($_.Name,'^\d+').Value)});$expected=1..($vers.Count);Add-Check 'Migration sequence' (if(($vers -join ',') -eq ($expected -join ',')){'PASS'}else{'FAIL'}) "count=$($vers.Count)"
if(Test-Path (Join-Path $Root '.env.example')){Add-Check 'Bootstrap environment documented' (if((Get-Content (Join-Path $Root '.env.example') -Raw) -match 'SCI_BOOTSTRAP_USERNAME'){'PASS'}else{'FAIL'})}
if(-not $SkipDatabase -and (Get-Command docker -ErrorAction SilentlyContinue)){
  $up=& docker compose ps --status running --services 2>&1;$running=($LASTEXITCODE -eq 0 -and ($up -contains 'postgres'));Add-Check 'PostgreSQL running' (if($running){'PASS'}else{'WARN'})
  if($running){$sql='SELECT 1;';$out=& docker compose exec -T postgres sh -lc 'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc "SELECT 1"' 2>&1;Add-Check 'PostgreSQL health' (if($LASTEXITCODE -eq 0 -and (($out -join '') -match '1')){'PASS'}else{'FAIL'})}
}else{Add-Check 'PostgreSQL diagnostics' 'SKIP' 'Docker non disponible ou SkipDatabase demandé'}
$report.Add('')|Out-Null;$report.Add("Failures: $fail")|Out-Null;$report.Add("Warnings: $warn")|Out-Null;$report.Add("Finished: $(Get-Date -Format 'o')")|Out-Null;$report|Set-Content $ReportPath -Encoding utf8
if($fail -gt 0){Write-Host 'DOCTOR_STATUS=FAIL';exit 1}else{Write-Host 'DOCTOR_STATUS=PASS';Write-Host "REPORT=$ReportPath";exit 0}
