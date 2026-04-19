<#
.SYNOPSIS
Dispatches GUI validation lanes.

.DESCRIPTION
Keeps GUI-specific automation scripts grouped under `scripts/gui/`.
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
  "contract" = "run_gui_contract.ps1"
  "suite" = "run_gui_suite.ps1"
  "aiv-smoke" = "run_gui_aiv_smoke.ps1"
  "aiv-suite" = "run_gui_aiv_suite.ps1"
}

if ([string]::IsNullOrWhiteSpace($Command) -or $Command -in @("-h", "--help", "-Help")) {
  Write-Host "Usage: scripts/gui.ps1 <contract|suite|aiv-smoke|aiv-suite> [args...]"
  exit 0
}

$scriptName = $commands[$Command]
if ($null -eq $scriptName) {
  throw "Unknown GUI command: $Command"
}

& (Join-Path $PSScriptRoot "gui/$scriptName") @Arguments
