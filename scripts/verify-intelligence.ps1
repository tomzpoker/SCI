$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
Write-Host "=== SCI FAMILY Intelligence ==="
$rustFiles = @(Get-ChildItem .\src -Recurse -File -ErrorAction Stop | Where-Object { $_.Extension -eq ".rs" })
$front = @(Get-ChildItem .\src -Recurse -File -ErrorAction Stop | Where-Object { $_.Extension -in @(".ts",".tsx",".js",".jsx") })
Write-Host "STATIC_AUDIT=PASS"
Write-Host "rust_source_files=$($rustFiles.Count)"
if ($front.Count -ne 0) { Write-Host "frontend_source=NOT_RUST_ONLY"; exit 1 }
Write-Host "frontend_source=RUST_ONLY"
$ui = Get-Content (Join-Path $repo "src\ui.rs") -Raw
if ($ui.Contains('"{format!')) { Write-Host "STATIC_AUDIT=FAIL"; Write-Host "RSX_NESTED_FORMAT=FOUND"; exit 1 }
$intelligence = Get-Content (Join-Path $repo "src\intelligence.rs") -Raw
if ($intelligence -match 'imap_port\s*:\s*u16') { Write-Host "STATIC_AUDIT=FAIL"; Write-Host "IMAP_PORT_U16=FOUND"; exit 1 }
if ($intelligence -match 'process_uploaded_document\(filename:String,content_type:String,data_base64:String') { Write-Host "STATIC_AUDIT=FAIL"; Write-Host "LEGACY_BASE64_UPLOAD=FOUND"; exit 1 }
if (!$intelligence.Contains('process_uploaded_document(mut upload: FileStream)')) { Write-Host "STATIC_AUDIT=FAIL"; Write-Host "FILESTREAM_UPLOAD_MISSING"; exit 1 }
if ($intelligence.Contains('UploadEnvelope')) { Write-Host "STATIC_AUDIT=FAIL"; Write-Host "UPLOAD_ENVELOPE_FOUND"; exit 1 }
$required = @("src\intelligence.rs","src\domain.rs","src\ui.rs","migrations\0005_intelligence.sql")
foreach ($f in $required) { if (!(Test-Path $f)) { Write-Host "MISSING=$f"; exit 1 } }
function Invoke-Gate([string]$Label,[scriptblock]$Command) {
    Write-Host "--- $Label ---"
    & $Command
    if ($LASTEXITCODE -ne 0) { Write-Host "RUNTIME_BUILD_GATES=FAIL"; exit $LASTEXITCODE }
}
Invoke-Gate "cargo fmt" { cargo fmt --all -- --check }
Invoke-Gate "cargo test --features server" { cargo test --features server }
Invoke-Gate "cargo check --features web" { cargo check --features web }
Invoke-Gate "cargo check wasm32 --features web" { cargo check --target wasm32-unknown-unknown --features web }
Write-Host "RUNTIME_BUILD_GATES=PASS"
