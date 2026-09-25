$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$sql=Get-ChildItem $root\migrations3_einvoicing.sql,$root\migrations5_ai_provider_runtime.sql | Get-Content -Raw
if($sql -match '(?i)\bDROP TABLE\b|\bTRUNCATE\b'){throw 'SQL destructive détecté'}
foreach($x in 'einvoice_providers','einvoice_events','einvoice_documents','einvoice_ereporting'){if($sql -notmatch [regex]::Escape($x)){throw "Objet S13 absent: $x"}}
foreach($x in 'ACKNOWLEDGED','DELIVERED','RECEIVED','REJECTED','ERROR'){if($sql -notmatch [regex]::Escape($x)){throw "Statut S13 absent: $x"}}
Write-Host 'S13_EINVOICING_STATIC_GATE=PASS'
