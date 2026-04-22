
<#
.SYNOPSIS
Run the mandatory agent preflight and local CI contract.

.DESCRIPTION
Runs mandatory guardrails, then optionally runs local CI.
#>

param(
  [switch]$SkipCi,
  [switch]$QuickCi,
  [switch]$FullCi,
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


if ($Help) {
  Write-Host "Usage: run_agent_request.ps1 [-SkipCi] [-QuickCi] [-FullCi]"
  Write-Host ""
  Write-Host "Run the mandatory agent preflight and optional local development checks."
  Write-Host "Default CI path: `./scripts/ci.ps1 smoke`."
  Write-Host "If -QuickCi is set, run `./scripts/ci.ps1 quick`."
  Write-Host "If -FullCi is set, run `./scripts/ci.ps1 local -SkipAgentPreflight`."
  Write-Host "If -SkipCi is set, skip both local CI paths."
  exit 0
}

if ($QuickCi -and $FullCi) {
  throw "QuickCi and FullCi are mutually exclusive."
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path

Write-Host "[agent_request] preflight"

& (Join-Path $rootDir "scripts/internal/agent/run_agent_preflight.ps1")

if (-not $SkipCi) {
  if ($FullCi) {
    & (Join-Path $rootDir "scripts/ci.ps1") local -SkipAgentPreflight
  } elseif ($QuickCi) {
    & (Join-Path $rootDir "scripts/ci.ps1") quick
  } else {
    & (Join-Path $rootDir "scripts/ci.ps1") smoke
  }
}

Write-Host "[agent_request] OK"
