
<#
.SYNOPSIS
Runs mandatory agent-request readiness checks in PowerShell.

.DESCRIPTION
PowerShell equivalent of `scripts/internal/agent/run_agent_ci_checks.sh`.
Runs the agent-facing guardrail checks.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


if ($Help) {
  Write-Host "Usage: scripts/internal/agent/run_agent_ci_checks.ps1"
  Write-Host ""
  Write-Host "Run agent-request readiness checks used by local CI conventions."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$checkDir = Join-Path $rootDir "scripts/internal/check"

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

  Write-Host ("[agent_ci] {0}" -f $Label)

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
  Invoke-Check -Label "development branch policy" -ScriptPath (Join-Path $checkDir "check_next_branch.ps1")
  Invoke-Check -Label "migration boundary guardrails" -ScriptPath (Join-Path $checkDir "check_migration_boundary.ps1")
  Invoke-Check -Label "script guardrails" -ScriptPath (Join-Path $checkDir "check_script_guardrails.ps1")
  Invoke-Check -Label "workflow toolchain pinning" -ScriptPath (Join-Path $checkDir "check_workflow_toolchain_pinning.ps1")
  Invoke-Check -Label "manual docs scope guard" -ScriptPath (Join-Path $checkDir "check_manual_docs_scope.ps1")
  Invoke-Check -Label "legacy app coupling guardrail" -ScriptPath (Join-Path $checkDir "check_legacy_app_coupling.ps1")
  Invoke-Check -Label "rust todo/todo guardrail (non-test only)" -ScriptPath (Join-Path $checkDir "check_rust_no_todos.ps1")

  # Dead dependency sweep is advisory and has no first-class PowerShell parity yet.
  Write-Host "[agent_ci] rust dead dependency/unused code sweep (advisory)"
  Write-Host "[agent_ci] INFO: advisory dead-dependency sweep is skipped in native PowerShell mode."

  Invoke-Check -Label "rust public docs guardrail" -ScriptPath (Join-Path $checkDir "check_rust_public_docs.ps1")
  Invoke-Check -Label "rust private docs guardrail" -ScriptPath (Join-Path $checkDir "check_rust_private_docs.ps1")
  Invoke-Check -Label "app_core dependency boundary" -ScriptPath (Join-Path $checkDir "check_app_core_dependency_boundary.ps1")
  Invoke-Check -Label "knowledge lint" -ScriptPath (Join-Path $checkDir "knowledge_lint.ps1")

  Write-Host "[agent_ci] OK"
  exit 0
} finally {
  Pop-Location
}
