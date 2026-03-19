<#
.SYNOPSIS
Runs the broad workspace compile gate.

.DESCRIPTION
Checks tests and binary targets across all workspace members. Use this after
workspace/package changes or when support-tool coverage matters locally.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "use_cargo_cache.ps1")

if ($Help) {
  Write-Host "Usage: scripts/devcheck_workspace.ps1"
  Write-Host "Run the broad compile/smoke gate for all workspace packages."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function Invoke-NativeStep {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Label,
    [Parameter(Mandatory = $true)]
    [scriptblock]$Command
  )

  & $Command
  if ($LASTEXITCODE -ne 0) {
    throw "[devcheck_workspace] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-SempalCargoCache
  Write-Host "[devcheck_workspace] cargo check --workspace --tests --bins"
  Invoke-NativeStep -Label "cargo check --workspace --tests --bins" -Command {
    Invoke-SempalCargo check --workspace --tests --bins
  }

  Write-Host "[devcheck_workspace] OK"
} finally {
  Pop-Location
}
