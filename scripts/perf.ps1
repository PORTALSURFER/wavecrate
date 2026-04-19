<#
.SYNOPSIS
Dispatches runtime performance helpers.

.DESCRIPTION
The detailed benchmark scripts live under `scripts/perf/`; this keeps the
top-level script list compact while leaving the perf tools intact.
#>

param(
  [Parameter(Position = 0)]
  [string]$Command,
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$Arguments
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$commands = @{
  "guard" = "run_perf_guard.ps1"
  "wheel-stability" = "run_perf_wheel_stability.ps1"
}

if ([string]::IsNullOrWhiteSpace($Command) -or $Command -in @("-h", "--help", "-Help")) {
  Write-Host "Usage: scripts/perf.ps1 <guard|wheel-stability> [args...]"
  exit 0
}

$scriptName = $commands[$Command]
if ($null -eq $scriptName) {
  throw "Unknown perf command: $Command"
}

& (Join-Path $PSScriptRoot "perf/$scriptName") @Arguments
