Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Runs script guardrail checks for agent-facing shell scripts.
#>

if (-not (Get-Command bash -ErrorAction SilentlyContinue)) {
  Write-Error "[guardrails] bash is required to run this check."
  exit 1
}

& bash scripts/check_script_guardrails.sh
