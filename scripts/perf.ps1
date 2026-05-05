<#
.SYNOPSIS
Dispatches runtime performance helpers.

.DESCRIPTION
The detailed benchmark scripts live under `scripts/internal/perf/`; this keeps
the top-level script list compact while leaving the perf tools intact.
#>

param(
  [Parameter(Position = 0)]
  [string]$Command,
  [switch]$Help,
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$Arguments
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path

$commands = @{
  "guard" = "run_perf_guard.ps1"
  "wheel-stability" = "run_perf_wheel_stability.ps1"
}

if ([string]::IsNullOrWhiteSpace($Command)) {
  Write-Host "Usage: scripts/perf.ps1 <guard|wheel-stability> [args...]"
  exit 0
}

if ($Help) {
  $Arguments = @("-Help") + $Arguments
}

$scriptName = $commands[$Command]
if ($null -eq $scriptName) {
  throw "Unknown perf command: $Command"
}

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "internal/perf/$scriptName") @Arguments
