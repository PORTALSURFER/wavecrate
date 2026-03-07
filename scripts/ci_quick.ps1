<#
.SYNOPSIS
Runs a fast local development test pass.

.DESCRIPTION
Executes the everyday validation loop for normal development by running the
project's standard unit, integration, and binary tests through cargo-nextest.
This intentionally skips clippy, rustdoc, benches, perf guards, and other
slower CI-parity checks reserved for `scripts/ci_local.ps1`.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($Help) {
  Write-Host "Usage: scripts/ci_quick.ps1"
  Write-Host "Run the fast local development test loop."
  Write-Host "For full CI parity, use `scripts/ci_local.ps1`."
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
    throw "[ci_quick] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Write-Host "[ci_quick] cargo nextest run --lib --bins --tests --no-fail-fast"
  Invoke-NativeStep -Label "cargo nextest run --lib --bins --tests --no-fail-fast" -Command {
    cargo nextest run --lib --bins --tests --no-fail-fast
  }

  Write-Host "[ci_quick] OK"
} finally {
  Pop-Location
}
