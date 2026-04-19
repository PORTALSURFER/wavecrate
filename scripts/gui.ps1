<#
.SYNOPSIS
Dispatches GUI validation lanes.

.DESCRIPTION
Keeps GUI-specific automation scripts grouped under `scripts/internal/gui/`.
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
  "contract" = "run_gui_contract.ps1"
  "suite" = "run_gui_suite.ps1"
  "aiv-smoke" = "run_gui_aiv_smoke.ps1"
  "aiv-suite" = "run_gui_aiv_suite.ps1"
}

if ([string]::IsNullOrWhiteSpace($Command)) {
  Write-Host "Usage: scripts/gui.ps1 <contract|suite|aiv-smoke|aiv-suite> [args...]"
  exit 0
}

if ($Help) {
  $Arguments = @("-Help") + $Arguments
}

$scriptName = $commands[$Command]
if ($null -eq $scriptName) {
  throw "Unknown GUI command: $Command"
}

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "internal/gui/$scriptName") @Arguments
