$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
& "$root\scripts\verify-s13-einvoicing.ps1"
& "$root\scripts\verify-s14-ai.ps1"
$migs=Get-ChildItem $root\migrations\*.sql | Where-Object {$_.Name -match '^(\d+)_'} | Sort-Object Name
$nums=$migs | ForEach-Object {[int]($_.BaseName -split '_',2)[0]}
for($i=1;$i -le $nums.Count;$i++){if($nums[$i-1] -ne $i){throw "Migration non continue: attendu $i, trouvé $($nums[$i-1])"}}
if($nums[-1] -ne 26){throw "Dernière migration attendue: 0026, trouvé $($nums[-1])"}
Write-Host 'S13_S14_FINAL_STATIC_GATE=PASS'
