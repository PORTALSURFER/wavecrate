Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Collects/evaluates wheel-latency stability evidence for perf guard threshold promotion.

.DESCRIPTION
PowerShell wrapper that delegates to the canonical bash implementation
(`scripts/run_perf_wheel_stability.sh`).
#>

param(
  [ValidateSet("collect", "evaluate", "collect-and-evaluate")]
  [string]$Mode = "collect-and-evaluate"
)

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  if (-not (Get-Command bash -ErrorAction SilentlyContinue)) {
    throw "[wheel_stability] ERROR: bash is required for scripts/run_perf_wheel_stability.sh"
  }
  & bash (Join-Path $rootDir "scripts/run_perf_wheel_stability.sh") $Mode
} finally {
  Pop-Location
}
