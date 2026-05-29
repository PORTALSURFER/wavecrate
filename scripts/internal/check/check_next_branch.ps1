<#
.SYNOPSIS
Verifies that the current repository is working directly on next.

.DESCRIPTION
Requires the current branch to be local `next` tracking `origin/next`. Agent
work should be committed and pushed directly to `next`, not to feature branches.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($Help) {
  Write-Host "Usage: scripts/internal/check/check_next_branch.ps1"
  Write-Host "Verify that the current branch is local next tracking origin/next."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
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
  throw "[branch_guard] Detached HEAD is not allowed. Use local '$expectedBranch'."
}

if ($branch -ne $expectedBranch) {
  throw "[branch_guard] Current branch must be '$expectedBranch'. Current branch: '$branch'."
}

$integrationUpstream = Invoke-GitText -Arguments @(
  "-C", $rootDir,
  "for-each-ref",
  "--format=%(upstream:short)",
  "refs/heads/$expectedBranch"
)

if ([string]::IsNullOrWhiteSpace($integrationUpstream)) {
  throw "[branch_guard] Local '$expectedBranch' must exist and track '$expectedUpstream'."
}

if ($integrationUpstream -ne $expectedUpstream) {
  throw "[branch_guard] Local '$expectedBranch' must track '$expectedUpstream'. Current upstream: '$integrationUpstream'."
}

Write-Host "[branch_guard] OK ($branch -> $integrationUpstream)"
