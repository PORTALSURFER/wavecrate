Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

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
  [switch]$Help
)

if ($Help) {
  Write-Host "Usage: run_agent_request.ps1 [-Updater Codex] [-MemoryMaxAgeHours 1] [-SkipCi]"
  Write-Host ""
  Write-Host "Run the mandatory agent preflight and optional full local CI gate."
  Write-Host "If -SkipCi is set, skip `./scripts/ci_local.sh --skip-agent-preflight`."
  exit 0
}

if ($MemoryMaxAgeHours -lt 0) {
  throw "MemoryMaxAgeHours must be >= 0."
}

if (-not (Get-Command bash -ErrorAction SilentlyContinue)) {
  throw "bash is required to run agent request preflight scripts."
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

Write-Host "[agent_request] updater=$Updater memory_max_age_hours=$MemoryMaxAgeHours"

& bash (Join-Path $rootDir "scripts/run_agent_preflight.sh") --refresh-memory --updater "$Updater" --memory-max-age-hours "$MemoryMaxAgeHours"

if (-not $SkipCi) {
  & bash (Join-Path $rootDir "scripts/ci_local.sh") --skip-agent-preflight
}

Write-Host "[agent_request] OK"
