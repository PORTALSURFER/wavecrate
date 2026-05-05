<#
.SYNOPSIS
Dispatches sandbox and diagnostic run helpers.

.DESCRIPTION
Collapses the old sandbox/log/bundle helper scripts into one public entrypoint.
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
  "sandbox" = "run_sandbox.ps1"
  "clean" = "clean_sandbox.ps1"
  "logs" = "latest_log.ps1"
  "bug-bundle" = "bug_bundle.ps1"
}

if ([string]::IsNullOrWhiteSpace($Command)) {
  Write-Host "Usage: scripts/run.ps1 <sandbox|clean|logs|bug-bundle> [args...]"
  exit 0
}

if ($Help) {
  $Arguments = @("-Help") + $Arguments
}

$scriptName = $commands[$Command]
if ($null -eq $scriptName) {
  throw "Unknown run command: $Command"
}

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "internal/run/$scriptName") @Arguments
