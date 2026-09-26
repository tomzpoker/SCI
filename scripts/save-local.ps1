# Configuration des dossiers
$sourceDir = "D:\SCI\DEV\sci-family-rust"
$outputDir = "D:\SCI\DEV"

# Format de date : AAAAMMJJ (ex: 20260923_sci-family-rust.zip)
$dateStr = Get-Date -Format "yyyyMMdd"
$zipName = "${dateStr}_sci-family-rust.zip"
$zipPath = Join-Path -Path $outputDir -ChildPath $zipName

# Dossier temporaire pour isoler les fichiers avant compression
$tempDir = Join-Path -Path $env:TEMP -ChildPath "sci-family-rust-backup"

if (Test-Path $tempDir) {
    Remove-Item -Path $tempDir -Recurse -Force
}

Write-Host "Copie des fichiers en excluant 'target'..." -ForegroundColor Cyan

# Copie avec exclusion de target
Get-ChildItem -Path $sourceDir -Recurse | Where-Object {
    $_.FullName -notmatch [regex]::Escape("\target\") -and
    $_.FullName -notmatch [regex]::Escape("\target")
} | ForEach-Object {
    $relativePath = $_.FullName.Substring($sourceDir.Length)
    $destinationPath = Join-Path -Path $tempDir -ChildPath $relativePath

    if ($_.PSIsContainer) {
        New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null
    } else {
        $parentDir = Split-Path -Path $destinationPath -Parent
        if (-not (Test-Path $parentDir)) {
            New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
        }
        Copy-Item -Path $_.FullName -Destination $destinationPath -Force
    }
}

Write-Host "Création de l'archive dans $outputDir..." -ForegroundColor Cyan

if (Test-Path $zipPath) {
    Remove-Item -Path $zipPath -Force
}

# Compression
Compress-Archive -Path "$tempDir\*" -DestinationPath $zipPath -CompressionLevel Optimal

# Nettoyage temporaire
Remove-Item -Path $tempDir -Recurse -Force

Write-Host "Archive créée avec succès : $zipPath" -ForegroundColor Green