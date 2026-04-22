
<#
.SYNOPSIS
Runs mandatory agent-facing preflight checks in PowerShell.

.DESCRIPTION
PowerShell equivalent of `scripts/internal/agent/run_agent_preflight.sh`.
Runs `run_agent_ci_checks.ps1`.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


if ($Help) {
  Write-Host "Usage: scripts/internal/agent/run_agent_preflight.ps1"
  Write-Host ""
  Write-Host "Run mandatory preflight checks for an agent request."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$checksScript = Join-Path $rootDir "scripts/internal/agent/run_agent_ci_checks.ps1"

$psRunner = Get-Command pwsh -ErrorAction SilentlyContinue
if ($null -eq $psRunner) {
  $psRunner = Get-Command powershell -ErrorAction SilentlyContinue
}
if ($null -eq $psRunner) {
  throw "[agent_preflight] Neither pwsh nor powershell is available to execute check scripts."
}
$psExe = $psRunner.Path

$args = @(
  "-NoProfile",
  "-File",
  $checksScript
)

& $psExe @args
if ($LASTEXITCODE -ne 0) {
  throw "[agent_preflight] run_agent_ci_checks.ps1 failed with exit code $LASTEXITCODE."
}

Write-Host "[agent_preflight] OK"
