<#
.SYNOPSIS
Runs the fast workspace-wide test pass.

.DESCRIPTION
Executes the quick nextest profile over all workspace members. Use this for
packaging and tooling changes after the app-focused lane is no longer enough.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "use_cargo_cache.ps1")

if ($Help) {
  Write-Host "Usage: scripts/ci_quick_workspace.ps1"
  Write-Host "Run the fast test loop for all workspace packages."
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
    throw "[ci_quick_workspace] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-SempalCargoCache
  Write-Host "[ci_quick_workspace] cargo nextest run --workspace --profile quick --all-targets"
  Invoke-NativeStep -Label "cargo nextest run --workspace --profile quick --all-targets" -Command {
    cargo nextest run --workspace --profile quick --all-targets
  }

  Write-Host "[ci_quick_workspace] OK"
} finally {
  Pop-Location
}
