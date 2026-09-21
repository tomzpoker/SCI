$ErrorActionPreference = "Stop"

$ProjectPath = "D:\SCI\DEV\sci-family-rust"
$Repo        = "tomzpoker/SCI"
$Visibility  = "private"

Set-Location $ProjectPath

Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host " SCI Family Pilot -> GitHub" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "Git n'est pas installé ou n'est pas dans le PATH."
}

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "GitHub CLI (gh) n'est pas installé ou n'est pas dans le PATH."
}

Write-Host "[1/7] Vérification de l'authentification GitHub..." -ForegroundColor Yellow
gh auth status

Write-Host "[2/7] Sécurisation du .gitignore..." -ForegroundColor Yellow
$gitignore = Join-Path $ProjectPath ".gitignore"
if (-not (Test-Path $gitignore)) {
    New-Item -ItemType File -Path $gitignore | Out-Null
}
$currentIgnore = Get-Content $gitignore -Raw -ErrorAction SilentlyContinue
foreach ($entry in @(".env", ".env.*", "target/", "node_modules/")) {
    if ($currentIgnore -notmatch [regex]::Escape($entry)) {
        Add-Content -Path $gitignore -Value $entry
    }
}
git rm --cached --ignore-unmatch .env 2>$null | Out-Null

Write-Host "[3/7] Initialisation Git..." -ForegroundColor Yellow
if (-not (Test-Path (Join-Path $ProjectPath ".git"))) {
    git init
}
git branch -M main

Write-Host "[4/7] Préparation du commit..." -ForegroundColor Yellow
git add .
$status = git status --porcelain
if ($status) {
    git commit -m "Initial SCI Family Pilot"
}
else {
    Write-Host "Aucun changement à committer." -ForegroundColor DarkGray
}

Write-Host "[5/7] Vérification du dépôt GitHub..." -ForegroundColor Yellow
$repoExists = $false
try {
    gh repo view $Repo *> $null
    if ($LASTEXITCODE -eq 0) {
        $repoExists = $true
    }
}
catch {
    $repoExists = $false
}

Write-Host "[6/7] Push vers $Repo..." -ForegroundColor Yellow
if (-not $repoExists) {
    gh repo create $Repo --$Visibility --source . --remote origin --push
}
else {
    $origin = git remote get-url origin 2>$null
    if (-not $origin) {
        git remote add origin "https://github.com/$Repo.git"
    }
    elseif ($origin -ne "https://github.com/$Repo.git") {
        git remote set-url origin "https://github.com/$Repo.git"
    }
    git push -u origin main
}

Write-Host "[7/7] Vérification finale..." -ForegroundColor Yellow
git remote -v
git status --short

Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host " PUSH TERMINE" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green
Write-Host "Repository : https://github.com/$Repo" -ForegroundColor Cyan
Write-Host ""
Write-Host "Le fichier .env reste local et est exclu du dépôt." -ForegroundColor Yellow
