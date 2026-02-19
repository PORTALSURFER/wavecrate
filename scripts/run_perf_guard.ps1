Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Runs the local performance guard benchmark suite.

.DESCRIPTION
PowerShell wrapper that delegates to the canonical bash implementation.
The guard is warning-only for latency drift and errors only on execution/report failures.
#>

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  if (-not (Get-Command bash -ErrorAction SilentlyContinue)) {
    throw "[perf_guard] ERROR: bash is required for scripts/run_perf_guard.sh"
  }
  & bash (Join-Path $rootDir "scripts/run_perf_guard.sh")
} finally {
  Pop-Location
}
