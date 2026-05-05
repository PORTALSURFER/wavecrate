<#
.SYNOPSIS
Dispatches the agent workflow helpers.

.DESCRIPTION
Keeps the top-level `scripts/` menu short while hiding the specialized
agent-preflight implementations under `scripts/internal/agent/`.
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
  "install-hooks" = "install_agent_preflight_hooks.sh"
  "request" = "run_agent_request.ps1"
  "preflight" = "run_agent_preflight.ps1"
  "checks" = "run_agent_ci_checks.ps1"
}

if ([string]::IsNullOrWhiteSpace($Command)) {
  Write-Host "Usage: scripts/agent.ps1 <request|preflight|checks|install-hooks> [args...]"
  exit 0
}

if ($Help) {
  $Arguments = @("-Help") + $Arguments
}

$scriptName = $commands[$Command]
if ($null -eq $scriptName) {
  throw "Unknown agent command: $Command"
}

if ($scriptName.EndsWith(".sh")) {
  $bash = Get-Command bash -ErrorAction SilentlyContinue
  if ($null -eq $bash) {
    throw "bash is required for agent command '$Command'."
  }
  & $bash.Path (Join-Path $PSScriptRoot "internal/agent/$scriptName") @Arguments
} else {
  & $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "internal/agent/$scriptName") @Arguments
}
