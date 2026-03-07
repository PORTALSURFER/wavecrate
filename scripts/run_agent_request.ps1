
<#
.SYNOPSIS
Run the mandatory agent preflight and local CI contract.

.DESCRIPTION
Refreshes MEMORY.md, runs mandatory guardrails, then optionally runs local CI.
#>

param(
  [string]$Updater = "Codex",
  [int]$MemoryMaxAgeHours = 1,
  [switch]$SkipCi,
  [switch]$FullCi,
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


if ($Help) {
  Write-Host "Usage: run_agent_request.ps1 [-Updater Codex] [-MemoryMaxAgeHours 1] [-SkipCi] [-FullCi]"
  Write-Host ""
  Write-Host "Run the mandatory agent preflight and optional local development checks."
  Write-Host "Default CI path: `./scripts/ci_quick.ps1`."
  Write-Host "If -FullCi is set, run `./scripts/ci_local.ps1 -SkipAgentPreflight` instead."
  Write-Host "If -SkipCi is set, skip both local CI paths."
  exit 0
}

if ($MemoryMaxAgeHours -lt 0) {
  throw "MemoryMaxAgeHours must be >= 0."
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

Write-Host "[agent_request] updater=$Updater memory_max_age_hours=$MemoryMaxAgeHours"

& (Join-Path $rootDir "scripts/run_agent_preflight.ps1") -RefreshMemory -Updater "$Updater" -MemoryMaxAgeHours "$MemoryMaxAgeHours"

if (-not $SkipCi) {
  if ($FullCi) {
    & (Join-Path $rootDir "scripts/ci_local.ps1") -SkipAgentPreflight
  } else {
    & (Join-Path $rootDir "scripts/ci_quick.ps1")
  }
}

Write-Host "[agent_request] OK"
