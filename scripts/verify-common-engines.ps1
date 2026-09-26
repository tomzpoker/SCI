$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$files = @("src\engines.rs","src\history.rs","src\lib.rs","src\server.rs","src\ui.rs","S02_US-0205.md","S02_US-0205_README.md","versions\S02_US-0205.md")
foreach($rel in $files){ if(!(Test-Path (Join-Path $root $rel))){ throw "Missing: $rel" } }
$s = Get-Content (Join-Path $root "src\engines.rs") -Raw
foreach($needle in @("Calcul","Event","Workflow","Bank","Vat","Document","Cash","Audit","EngineContext","legal_entity_id","list_common_engines")){ if($s -notmatch [regex]::Escape($needle)){ throw "Missing common engine marker: $needle" } }
Write-Host "COMMON_ENGINES_STATIC_GATE=PASS"
