<#
.SYNOPSIS
Runs the lightest app-only compile gate.

.DESCRIPTION
Checks the main library plus the `sempal` application binary without building
support-tool bins or test targets. Use this before `scripts/devcheck.ps1` when
you are only changing normal app/runtime code.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "use_cargo_cache.ps1")

if ($Help) {
  Write-Host "Usage: scripts/devcheck_app.ps1"
  Write-Host "Run the lightest app-only compile gate."
  Write-Host "Use this before `scripts/devcheck.ps1` when you are only changing the main app."
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
    throw "[devcheck_app] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-SempalCargoCache
  Write-Host "[devcheck_app] cargo check -p sempal --lib --bin sempal"
  Invoke-NativeStep -Label "cargo check -p sempal --lib --bin sempal" -Command {
    cargo check -p sempal --lib --bin sempal
  }

  Write-Host "[devcheck_app] OK"
} finally {
  Pop-Location
}
