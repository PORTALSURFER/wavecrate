<#
.SYNOPSIS
Verifies that the current repository uses next as its integration branch.

.DESCRIPTION
Allows feature branches for PR work, but verifies that local `next` exists and
tracks `origin/next`. When the current branch is `next`, it must track
`origin/next` directly.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($Help) {
  Write-Host "Usage: scripts/internal/check/check_next_branch.ps1"
  Write-Host "Verify that local next tracks origin/next; feature branches are allowed for PR work."
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
  throw "[branch_guard] Detached HEAD is not allowed. Use local '$expectedBranch' or a feature branch."
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

if ($branch -eq $expectedBranch) {
  Write-Host "[branch_guard] OK ($branch -> $integrationUpstream)"
} else {
  Write-Host "[branch_guard] OK (feature branch '$branch', base $expectedBranch -> $integrationUpstream)"
}
