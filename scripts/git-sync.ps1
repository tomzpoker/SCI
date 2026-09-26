[CmdletBinding()]
param(
  [ValidateSet('push-main','push-current','create-branch','push-new-branch','pull-current','sync','release')]
  [string]$Action='sync',
  [string]$Branch,
  [string]$Message='SCI Family Pilot release update'
)
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Set-Location $Root
function Git([string[]]$Args){& git @Args;if($LASTEXITCODE -ne 0){throw "git $($Args -join ' ') a échoué."}}
if(-not(Get-Command git -ErrorAction SilentlyContinue)){throw 'git introuvable.'}
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'verify-git-hygiene.ps1')
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
if(-not(Test-Path '.git')){Git @('init')}
$remote=(git remote get-url origin 2>$null)
if(-not $remote){Write-Host 'Aucun remote origin : configure-le avant push.'}
$current=(git branch --show-current).Trim()
if([string]::IsNullOrWhiteSpace($Branch)){$Branch=$current}
switch($Action){
 'push-main' { Git @('switch','main');Git @('pull','--ff-only','origin','main');Git @('add','.');Git @('commit','-m',$Message);Git @('push','-u','origin','main') }
 'push-current' { Git @('add','.');Git @('commit','-m',$Message);Git @('push','-u','origin',$Branch) }
 'create-branch' { Git @('switch','-c',$Branch) }
 'push-new-branch' { Git @('switch','-c',$Branch);Git @('add','.');Git @('commit','-m',$Message);Git @('push','-u','origin',$Branch) }
 'pull-current' { Git @('pull','--ff-only','origin',$Branch) }
 'sync' { Git @('fetch','origin','--prune');Git @('pull','--ff-only','origin',$Branch);Git @('status','--short') }
 'release' { Git @('fetch','origin','--prune');Git @('pull','--ff-only','origin',$Branch);Git @('add','.');Git @('commit','-m',$Message);Git @('push','-u','origin',$Branch) }
}
Write-Host "GIT_SYNC_STATUS=PASS action=$Action branch=$Branch" -ForegroundColor Green
