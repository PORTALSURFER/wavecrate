<#
.SYNOPSIS
Fast-forwards both development repos and runs Sempal in release sandbox mode.

.DESCRIPTION
Verifies that the main repo and `vendor/radiant` are both on local `next`
tracking `origin/next`, requires both worktrees to be clean, pulls the latest
remote commits with `git pull --ff-only origin next`, then delegates to
`scripts/run_sandbox.ps1`.

This script accepts the same sandbox options as `scripts/run_sandbox.ps1`. Any
trailing app arguments are forwarded to the release run after a `--` separator.
#>

param(
  [string]$Dir,
  [string]$Name,
  [switch]$Temp,
  [switch]$Clean,
  [switch]$WriteDb,
  [switch]$AllowUserLibraryDbWrite,
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($Help) {
  Write-Host "Usage: scripts/pull_and_run_release.ps1 [-Dir <path> | -Name <name> | -Temp] [-Clean] [-WriteDb] [-AllowUserLibraryDbWrite] [-- <app args...>]"
  Write-Host "Fast-forward the main repo and vendor/radiant from origin/next, then run scripts/run_sandbox.ps1."
  Write-Host "Both repos must be clean and on local next tracking origin/next."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$radiantDir = Join-Path $rootDir "vendor/radiant"
$runSandboxScript = Join-Path $rootDir "scripts/run_sandbox.ps1"

function Invoke-GitCommand {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoPath,
    [Parameter(Mandatory = $true)]
    [string[]]$GitArgs
  )

  $result = & git -C $RepoPath @GitArgs
  if ($LASTEXITCODE -ne 0) {
    $joinedArgs = $GitArgs -join " "
    throw "[pull_and_run_release] git $joinedArgs failed for $RepoPath with exit code $LASTEXITCODE."
  }
  return $result
}

function Assert-RepoExists {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoPath,
    [Parameter(Mandatory = $true)]
    [string]$Label
  )

  if (-not (Test-Path -LiteralPath $RepoPath)) {
    throw "[pull_and_run_release] Missing $Label repo at $RepoPath."
  }
}

function Assert-CleanRepo {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoPath,
    [Parameter(Mandatory = $true)]
    [string]$Label
  )

  $statusLines = Invoke-GitCommand -RepoPath $RepoPath -GitArgs @("status", "--short")
  if ($statusLines.Count -gt 0) {
    $details = ($statusLines | ForEach-Object { "  $_" }) -join [Environment]::NewLine
    throw "[pull_and_run_release] $Label repo is not clean:`n$details"
  }
}

function Assert-TrackedNextRepo {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoPath,
    [Parameter(Mandatory = $true)]
    [string]$Label
  )

  $branch = (Invoke-GitCommand -RepoPath $RepoPath -GitArgs @("rev-parse", "--abbrev-ref", "HEAD")).Trim()
  if ($branch -ne "next") {
    throw "[pull_and_run_release] $Label repo must be on local next, found '$branch'."
  }

  $upstream = (Invoke-GitCommand -RepoPath $RepoPath -GitArgs @("rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}")).Trim()
  if ($upstream -ne "origin/next") {
    throw "[pull_and_run_release] $Label repo must track origin/next, found '$upstream'."
  }
}

function Sync-Repo {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoPath,
    [Parameter(Mandatory = $true)]
    [string]$Label
  )

  Assert-RepoExists -RepoPath $RepoPath -Label $Label
  Assert-CleanRepo -RepoPath $RepoPath -Label $Label
  Assert-TrackedNextRepo -RepoPath $RepoPath -Label $Label

  Write-Host "[pull_and_run_release] Pulling $Label repo from origin/next"
  Invoke-GitCommand -RepoPath $RepoPath -GitArgs @("pull", "--ff-only", "origin", "next") | Out-Null
}

$runSandboxArgs = @()
if (-not [string]::IsNullOrWhiteSpace($Dir)) {
  $runSandboxArgs += @("-Dir", $Dir)
}
if (-not [string]::IsNullOrWhiteSpace($Name)) {
  $runSandboxArgs += @("-Name", $Name)
}
if ($Temp) {
  $runSandboxArgs += "-Temp"
}
if ($Clean) {
  $runSandboxArgs += "-Clean"
}
if ($WriteDb) {
  $runSandboxArgs += "-WriteDb"
}
if ($AllowUserLibraryDbWrite) {
  $runSandboxArgs += "-AllowUserLibraryDbWrite"
}

Sync-Repo -RepoPath $rootDir -Label "main"
Sync-Repo -RepoPath $radiantDir -Label "vendor/radiant"

Write-Host "[pull_and_run_release] Starting release sandbox run"
if ($args.Count -gt 0) {
  & $runSandboxScript @runSandboxArgs -- @args
} else {
  & $runSandboxScript @runSandboxArgs
}

exit $LASTEXITCODE
