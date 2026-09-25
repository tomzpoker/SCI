$ErrorActionPreference = "Stop"
$ProjectPath = Split-Path -Parent $PSScriptRoot
$Repo = "tomzpoker/SCI"
Set-Location $ProjectPath
if (-not (Get-Command git -ErrorAction SilentlyContinue)) { throw "Git n'est pas installé." }
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw "GitHub CLI (gh) n'est pas installé ou n'est pas dans le PATH." }
gh auth status
if (-not (Test-Path ".git")) { git init }
git branch -M main
$gitignore = ".gitignore"
if (-not (Test-Path $gitignore)) { New-Item -ItemType File $gitignore | Out-Null }
$ignore = Get-Content $gitignore -Raw -ErrorAction SilentlyContinue
foreach($entry in @('.env','.env.*','target/')) { if($ignore -notmatch [regex]::Escape($entry)){Add-Content $gitignore $entry} }
git rm --cached --ignore-unmatch .env 2>$null | Out-Null
git add .
if (git status --porcelain) { git commit -m "SCI Family Pilot 0.5.0" | Out-Host }
if (-not (gh repo view $Repo 2>$null)) {
  gh repo create $Repo --private --source . --remote origin --push
} else {
  $origin = git remote get-url origin 2>$null
  if(-not $origin){git remote add origin "https://github.com/$Repo.git"}
  elseif($origin -ne "https://github.com/$Repo.git"){git remote set-url origin "https://github.com/$Repo.git"}
  git push -u origin main
}
git status --short
git remote -v
Write-Host "Repository : https://github.com/$Repo" -ForegroundColor Cyan
