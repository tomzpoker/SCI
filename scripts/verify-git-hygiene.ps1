[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
if(-not (Get-Command git -ErrorAction SilentlyContinue)){throw 'git introuvable.'}
if(-not (Test-Path '.git')){Write-Host 'GIT_HYGIENE_STATUS=SKIP (pas de dépôt Git local)';exit 0}
$tracked=@(git ls-files)
$forbidden=@('.env','*.env.local','*.env.production','*.dump','*.bak','*.sqlite','*.db','.sci-inbox/*','snapshots/*','documents_real/*','data/real/*')
$badPaths=@()
foreach($f in $tracked){
  foreach($pattern in $forbidden){if($f -like $pattern){$badPaths += $f;break}}
}
if($badPaths.Count){$badPaths|ForEach-Object{Write-Host "FORBIDDEN_TRACKED_FILE=$_"};throw 'Fichiers locaux/secrets interdits suivis par Git.'}
$secretPatterns=@(
  '(?i)api[_-]?key\s*[:=]\s*["''][^"'']{16,}["'']',
  '(?i)password\s*[:=]\s*["''][^"'']{8,}["'']',
  '(?i)secret\s*[:=]\s*["''][^"'']{12,}["'']',
  '(?i)bearer\s+[a-z0-9\._\-]{20,}',
  '(?i)ghp_[a-z0-9]{20,}',
  '(?i)github_pat_[a-z0-9_]{20,}',
  '(?i)sk-[a-z0-9]{20,}',
  '(?i)AKIA[0-9A-Z]{16}'
)
$extensions=@('.rs','.toml','.ps1','.sh','.md','.txt','.json','.sql','.yml','.yaml')
$hits=New-Object System.Collections.Generic.List[string]
foreach($f in $tracked){
  if($extensions -notcontains ([IO.Path]::GetExtension($f).ToLowerInvariant())){continue}
  $full=Join-Path $Root $f
  if(-not(Test-Path $full)){continue}
  $text=Get-Content $full -Raw -ErrorAction SilentlyContinue
  foreach($rx in $secretPatterns){if([regex]::IsMatch($text,$rx)){ $hits.Add("$f :: $rx");break}}
}
if($hits.Count){$hits|ForEach-Object{Write-Host "SECRET_PATTERN=$_"};throw 'Motif de secret détecté.'}
Write-Host "GIT_HYGIENE_FILES=$($tracked.Count)"
Write-Host 'GIT_HYGIENE_STATUS=PASS' -ForegroundColor Green
