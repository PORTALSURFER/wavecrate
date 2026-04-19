<#
.SYNOPSIS
Dispatches the agent workflow helpers.

.DESCRIPTION
Keeps the top-level `scripts/` menu short while preserving the specialized
agent-preflight scripts under `scripts/agent/`.
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
  "request" = "run_agent_request.ps1"
  "preflight" = "run_agent_preflight.ps1"
  "checks" = "run_agent_ci_checks.ps1"
}

if ([string]::IsNullOrWhiteSpace($Command) -or $Command -in @("-h", "--help", "-Help")) {
  Write-Host "Usage: scripts/agent.ps1 <request|preflight|checks> [args...]"
  exit 0
}

$scriptName = $commands[$Command]
if ($null -eq $scriptName) {
  throw "Unknown agent command: $Command"
}

& (Join-Path $PSScriptRoot "agent/$scriptName") @Arguments
