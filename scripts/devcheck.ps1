<#
.SYNOPSIS
Runs the fastest local compile/smoke check.

.DESCRIPTION
Executes the smallest useful development gate by type-checking library, test,
and binary targets without running the test suite. Use this during normal edit
loops, then escalate to `scripts/ci_quick.ps1` before commit and
`scripts/ci_local.ps1` for full CI parity.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "use_cargo_cache.ps1")

if ($Help) {
  Write-Host "Usage: scripts/devcheck.ps1"
  Write-Host "Run the fastest local compile/smoke gate."
  Write-Host "For fast test coverage, use `scripts/ci_quick.ps1`."
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
    throw "[devcheck] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-SempalCargoCache
  Write-Host "[devcheck] branch policy"
  Invoke-NativeStep -Label "branch policy" -Command {
    & (Join-Path $PSScriptRoot "check_next_branch.ps1")
  }

  Write-Host "[devcheck] cargo check -p sempal --tests --bins"
  Invoke-NativeStep -Label "cargo check -p sempal --tests --bins" -Command {
    cargo check -p sempal --tests --bins
  }

  Write-Host "[devcheck] OK"
} finally {
  Pop-Location
}
