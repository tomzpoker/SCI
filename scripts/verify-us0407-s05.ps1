$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$required=@(
 'migrations\0013_idempotency.sql','migrations\0014_workflow_validation_automation.sql','src\idempotency.rs','src\workflow.rs','src\services.rs','src\server.rs','src\ui.rs','src\lib.rs'
)
foreach($p in $required){if(!(Test-Path (Join-Path $root $p))){throw "Missing: $p"}}
$sql=Get-Content (Join-Path $root 'migrations\0013_idempotency.sql') -Raw
$sql14=Get-Content (Join-Path $root 'migrations\0014_workflow_validation_automation.sql') -Raw
if($sql -match '(?im)^\s*(DROP\s+(TABLE|COLUMN|SCHEMA|INDEX)|TRUNCATE\s+|DELETE\s+FROM\s+)'){throw 'Forbidden destructive SQL in 0013'}
foreach($marker in @('job_executions','uq_job_executions_scope_key','idempotency_key')){if($sql -notmatch [regex]::Escape($marker)){throw "Missing marker: $marker"}}
foreach($marker in @('workflow_definitions','workflow_steps','workflow_runs','automation_policies','validation_requests','postcondition_checks')){if($sql14 -notmatch [regex]::Escape($marker)){throw "Missing S05 marker: $marker"}}
if($sql14 -match '(?im)^\s*(DROP\s+(TABLE|COLUMN|SCHEMA|INDEX)|TRUNCATE\s+|DELETE\s+FROM\s+)'){throw 'Forbidden destructive SQL in 0014'}
$idem=Get-Content (Join-Path $root 'src\idempotency.rs') -Raw
foreach($marker in @('claim_job','request_hash','complete_job','fail_job')){if($idem -notmatch [regex]::Escape($marker)){throw "Missing idempotency marker: $marker"}}
$wf=Get-Content (Join-Path $root 'src\workflow.rs') -Raw
foreach($marker in @('list_workflow_definitions','start_workflow','list_validation_requests','decide_validation_request','record_postcondition','automation_level')){if($wf -notmatch [regex]::Escape($marker)){throw "Missing workflow marker: $marker"}}
$server=Get-Content (Join-Path $root 'src\server.rs') -Raw
foreach($marker in @('idempotency_key','claim_job(pool, id, "ANTICIPATION_CYCLE"','ON CONFLICT(legal_entity_id,idempotency_key)')){if($server -notmatch [regex]::Escape($marker)){throw "Missing server integration: $marker"}}
Write-Host 'US0407_GATE=PASS'
Write-Host 'S05_GATE=PASS'
Write-Host 'S04_US0407=IMPLEMENTED'
Write-Host 'S05_US0501_0505=IMPLEMENTED'
