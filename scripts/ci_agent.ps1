<#
.SYNOPSIS
Compatibility wrapper for the agent-safe validation lane.

.DESCRIPTION
Preserves the legacy `scripts/ci_agent.ps1` entrypoint while delegating to the
canonical `scripts/ci.ps1 agent` dispatcher.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "ci.ps1") agent @args
exit $LASTEXITCODE
