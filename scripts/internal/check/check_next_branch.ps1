<#
.SYNOPSIS
Verifies that the current repository uses main as its base branch.

.DESCRIPTION
Allows feature branches for PR work, but verifies that local `main` exists and
tracks `origin/main`. When the current branch is `main`, it must track
`origin/main` directly.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($Help) {
  Write-Host "Usage: scripts/internal/check/check_next_branch.ps1"
  Write-Host "Verify that local main tracks origin/main; feature branches are allowed for PR work."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$expectedBranch = "main"
$expectedUpstream = "origin/main"

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

if ($branch -eq "next") {
  throw "[branch_guard] Local 'next' is retired. Use '$expectedBranch' as the base branch and feature branches for PR work."
}

$mainUpstream = Invoke-GitText -Arguments @(
  "-C", $rootDir,
  "for-each-ref",
  "--format=%(upstream:short)",
  "refs/heads/$expectedBranch"
)

if ([string]::IsNullOrWhiteSpace($mainUpstream)) {
  throw "[branch_guard] Local '$expectedBranch' must exist and track '$expectedUpstream'."
}

if ($mainUpstream -ne $expectedUpstream) {
  throw "[branch_guard] Local '$expectedBranch' must track '$expectedUpstream'. Current upstream: '$mainUpstream'."
}

if ($branch -eq $expectedBranch) {
  Write-Host "[branch_guard] OK ($branch -> $mainUpstream)"
} else {
  Write-Host "[branch_guard] OK (feature branch '$branch', base $expectedBranch -> $mainUpstream)"
}
