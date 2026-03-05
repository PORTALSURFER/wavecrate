
<#
.SYNOPSIS
Runs mandatory agent-facing preflight checks in PowerShell.

.DESCRIPTION
PowerShell equivalent of `scripts/run_agent_preflight.sh`.
Refreshes MEMORY.md (default) and runs `run_agent_ci_checks.ps1`.
#>

param(
  [switch]$RefreshMemory = $true,
  [switch]$NoRefresh,
  [string]$Updater = "Codex",
  [int]$MemoryMaxAgeHours = 1,
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


if ($Help) {
  Write-Host "Usage: scripts/run_agent_preflight.ps1 [-RefreshMemory] [-NoRefresh] [-Updater Codex] [-MemoryMaxAgeHours 1]"
  Write-Host ""
  Write-Host "Run mandatory preflight checks for an agent request."
  exit 0
}

if (-not $PSBoundParameters.ContainsKey("Updater")) {
  $envUpdater = [Environment]::GetEnvironmentVariable("AGENT_PREFLIGHT_UPDATER")
  if (-not [string]::IsNullOrWhiteSpace($envUpdater)) {
    $Updater = $envUpdater
  }
}
if (-not $PSBoundParameters.ContainsKey("MemoryMaxAgeHours")) {
  $envMaxAge = [Environment]::GetEnvironmentVariable("AGENT_PREFLIGHT_MEMORY_MAX_AGE_HOURS")
  if (-not [string]::IsNullOrWhiteSpace($envMaxAge)) {
    $parsedMaxAge = 0
    if (-not [int]::TryParse($envMaxAge, [ref]$parsedMaxAge)) {
      throw "[agent_preflight] AGENT_PREFLIGHT_MEMORY_MAX_AGE_HOURS must be a non-negative integer."
    }
    $MemoryMaxAgeHours = $parsedMaxAge
  }
}

if ($NoRefresh) {
  $RefreshMemory = $false
}
if ([string]::IsNullOrWhiteSpace($Updater)) {
  throw "[agent_preflight] Updater must be non-empty."
}
if ($MemoryMaxAgeHours -lt 0) {
  throw "[agent_preflight] MemoryMaxAgeHours must be >= 0."
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$scriptsDir = Join-Path $rootDir "scripts"
$checksScript = Join-Path $scriptsDir "run_agent_ci_checks.ps1"

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
  $checksScript,
  "-Updater",
  $Updater,
  "-RequiredUpdater",
  $Updater,
  "-MemoryMaxAgeHours",
  "$MemoryMaxAgeHours"
)
if ($RefreshMemory) {
  $args += "-RefreshMemory"
}

& $psExe @args
if ($LASTEXITCODE -ne 0) {
  throw "[agent_preflight] run_agent_ci_checks.ps1 failed with exit code $LASTEXITCODE."
}

Write-Host "[agent_preflight] OK"
