
<#
.SYNOPSIS
Runs mandatory agent-request readiness checks in PowerShell.

.DESCRIPTION
PowerShell equivalent of `scripts/run_agent_ci_checks.sh`.
Runs guardrail checks, optionally refreshes MEMORY.md, and validates
MEMORY.md freshness/updater ownership.
#>

param(
  [switch]$RefreshMemory,
  [string]$Updater = "Codex",
  [string]$RequiredUpdater = "",
  [int]$MemoryMaxAgeHours = 24,
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


if ($Help) {
  Write-Host "Usage: scripts/run_agent_ci_checks.ps1 [-RefreshMemory] [-Updater Codex] [-RequiredUpdater <name>] [-MemoryMaxAgeHours 24]"
  Write-Host ""
  Write-Host "Run agent-request readiness checks used by local CI conventions."
  exit 0
}

if (-not $PSBoundParameters.ContainsKey("MemoryMaxAgeHours")) {
  $envMaxAge = [Environment]::GetEnvironmentVariable("AGENT_CI_MEMORY_MAX_AGE_HOURS")
  if (-not [string]::IsNullOrWhiteSpace($envMaxAge)) {
    $parsedMaxAge = 0
    if (-not [int]::TryParse($envMaxAge, [ref]$parsedMaxAge)) {
      throw "[agent_ci] AGENT_CI_MEMORY_MAX_AGE_HOURS must be a non-negative integer."
    }
    $MemoryMaxAgeHours = $parsedMaxAge
  }
}
if (-not $PSBoundParameters.ContainsKey("RequiredUpdater")) {
  $RequiredUpdater = [Environment]::GetEnvironmentVariable("AGENT_CI_REQUIRED_UPDATER")
  if ($null -eq $RequiredUpdater) {
    $RequiredUpdater = ""
  }
}

if ([string]::IsNullOrWhiteSpace($Updater)) {
  throw "[agent_ci] Updater must be non-empty."
}
if ($MemoryMaxAgeHours -lt 0) {
  throw "[agent_ci] MemoryMaxAgeHours must be >= 0."
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$scriptsDir = Join-Path $rootDir "scripts"

$psRunner = Get-Command pwsh -ErrorAction SilentlyContinue
if ($null -eq $psRunner) {
  $psRunner = Get-Command powershell -ErrorAction SilentlyContinue
}
if ($null -eq $psRunner) {
  throw "[agent_ci] Neither pwsh nor powershell is available to execute check scripts."
}
$psExe = $psRunner.Path

function Invoke-Check {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Label,
    [Parameter(Mandatory = $true)]
    [string]$ScriptPath,
    [hashtable]$EnvVars = @{}
  )

  if (-not (Test-Path -LiteralPath $ScriptPath)) {
    throw "[agent_ci] Missing check script: $ScriptPath"
  }

  $details = @()
  if ($EnvVars.ContainsKey("MEMORY_MAX_AGE_HOURS")) {
    $details += "max_age=$($EnvVars["MEMORY_MAX_AGE_HOURS"])h"
  }
  if ($EnvVars.ContainsKey("MEMORY_REQUIRED_UPDATER") -and -not [string]::IsNullOrWhiteSpace($EnvVars["MEMORY_REQUIRED_UPDATER"])) {
    $details += "required_updater=$($EnvVars["MEMORY_REQUIRED_UPDATER"])"
  }

  if ($details.Count -gt 0) {
    Write-Host ("[agent_ci] {0} ({1})" -f $Label, ($details -join " "))
  } else {
    Write-Host ("[agent_ci] {0}" -f $Label)
  }

  $previous = @{}
  foreach ($key in $EnvVars.Keys) {
    $previous[$key] = [Environment]::GetEnvironmentVariable($key)
    [Environment]::SetEnvironmentVariable($key, [string]$EnvVars[$key])
  }

  try {
    & $psExe -NoProfile -File $ScriptPath
    if ($LASTEXITCODE -ne 0) {
      throw "[agent_ci] Check failed ($Label) with exit code $LASTEXITCODE."
    }
  } finally {
    foreach ($key in $EnvVars.Keys) {
      [Environment]::SetEnvironmentVariable($key, $previous[$key])
    }
  }
}

Push-Location $rootDir
try {
  if ($RefreshMemory) {
    & $psExe -NoProfile -File (Join-Path $scriptsDir "refresh_memory_md.ps1") -Updater $Updater
    if ($LASTEXITCODE -ne 0) {
      throw "[agent_ci] refresh_memory_md.ps1 failed with exit code $LASTEXITCODE."
    }
  }

  Invoke-Check -Label "memory log must be fresh (agent mode)" -ScriptPath (Join-Path $scriptsDir "check_memory_log.ps1") -EnvVars @{
    MEMORY_MAX_AGE_HOURS = "$MemoryMaxAgeHours"
    MEMORY_REQUIRED_UPDATER = "$RequiredUpdater"
  }
  Invoke-Check -Label "migration boundary guardrails" -ScriptPath (Join-Path $scriptsDir "check_migration_boundary.ps1")
  Invoke-Check -Label "script guardrails" -ScriptPath (Join-Path $scriptsDir "check_script_guardrails.ps1")
  Invoke-Check -Label "workflow toolchain pinning" -ScriptPath (Join-Path $scriptsDir "check_workflow_toolchain_pinning.ps1")
  Invoke-Check -Label "high-visibility guardrail score alignment" -ScriptPath (Join-Path $scriptsDir "check_quality_score_drift.ps1")
  Invoke-Check -Label "manual docs scope guard" -ScriptPath (Join-Path $scriptsDir "check_manual_docs_scope.ps1")
  Invoke-Check -Label "legacy app coupling guardrail" -ScriptPath (Join-Path $scriptsDir "check_legacy_app_coupling.ps1")
  Invoke-Check -Label "rust todo/todo guardrail (non-test only)" -ScriptPath (Join-Path $scriptsDir "check_rust_no_todos.ps1")

  # Dead dependency sweep is advisory and has no first-class PowerShell parity yet.
  Write-Host "[agent_ci] rust dead dependency/unused code sweep (advisory)"
  Write-Host "[agent_ci] INFO: advisory dead-dependency sweep is skipped in native PowerShell mode."

  Invoke-Check -Label "rust public docs guardrail" -ScriptPath (Join-Path $scriptsDir "check_rust_public_docs.ps1")
  Invoke-Check -Label "rust private docs guardrail" -ScriptPath (Join-Path $scriptsDir "check_rust_private_docs.ps1")
  Invoke-Check -Label "app_core dependency boundary" -ScriptPath (Join-Path $scriptsDir "check_app_core_dependency_boundary.ps1")
  Invoke-Check -Label "knowledge lint" -ScriptPath (Join-Path $scriptsDir "knowledge_lint.ps1")

  Write-Host "[agent_ci] OK"
  exit 0
} finally {
  Pop-Location
}
