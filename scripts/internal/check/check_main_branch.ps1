<#
.SYNOPSIS
Verifies that the current repository uses main as its integration branch.

.DESCRIPTION
Requires local `main` to track `origin/main`. Agent work should be committed and
pushed directly to `main` for ordinary Wavecrate work.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($Help) {
  Write-Host "Usage: scripts/internal/check/check_main_branch.ps1"
  Write-Host "Verify that local main tracks origin/main."
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
