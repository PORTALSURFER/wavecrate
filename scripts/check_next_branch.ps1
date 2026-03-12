<#
.SYNOPSIS
Verifies that the current repository uses the shared development branch.

.DESCRIPTION
Fails unless the repository rooted at the current workspace is on local
`next` and that branch tracks `origin/next`. This keeps sempal development on
the agreed branch and gives hooks and validation scripts a single branch-policy
entrypoint.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($Help) {
  Write-Host "Usage: scripts/check_next_branch.ps1"
  Write-Host "Fail unless the current repository is on local next tracking origin/next."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$expectedBranch = "next"
$expectedUpstream = "origin/next"

function Invoke-GitText {
  param(
    [Parameter(Mandatory = $true)]
    [string[]]$Arguments
  )

  $output = & git @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "[branch_guard] git $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
  }

  return ($output | Out-String).Trim()
}

$branch = Invoke-GitText -Arguments @(
  "-C", $rootDir,
  "rev-parse",
  "--abbrev-ref",
  "HEAD"
)

if ($branch -eq "HEAD") {
  throw "[branch_guard] Detached HEAD is not allowed. Switch this repository to local '$expectedBranch'."
}

if ($branch -ne $expectedBranch) {
  throw "[branch_guard] Development must happen on '$expectedBranch'. Current branch: '$branch'."
}

$upstream = Invoke-GitText -Arguments @(
  "-C", $rootDir,
  "for-each-ref",
  "--format=%(upstream:short)",
  "refs/heads/$branch"
)

if ([string]::IsNullOrWhiteSpace($upstream)) {
  throw "[branch_guard] Branch '$branch' has no upstream. Set it to '$expectedUpstream'."
}

if ($upstream -ne $expectedUpstream) {
  throw "[branch_guard] Branch '$branch' must track '$expectedUpstream'. Current upstream: '$upstream'."
}

Write-Host "[branch_guard] OK ($branch -> $upstream)"
