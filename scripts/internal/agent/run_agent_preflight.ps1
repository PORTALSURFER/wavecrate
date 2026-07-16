
<#
.SYNOPSIS
Runs mandatory agent-facing preflight checks in PowerShell.

.DESCRIPTION
PowerShell equivalent of `scripts/internal/agent/run_agent_preflight.sh`.
Runs `run_agent_ci_checks.ps1` under the same repository-local single-flight
contract as the Bash preflight runner.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


if ($Help) {
  Write-Host "Usage: scripts/internal/agent/run_agent_preflight.ps1"
  Write-Host ""
  Write-Host "Run mandatory full preflight checks for an agent request. Concurrent invocations coalesce to one owner."
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

$stateDir = $env:WAVECRATE_AGENT_PREFLIGHT_STATE_DIR
if ([string]::IsNullOrWhiteSpace($stateDir)) {
  $gitCommonDir = (& git -C $rootDir rev-parse --git-common-dir).Trim()
  if ([System.IO.Path]::IsPathRooted($gitCommonDir)) {
    $stateDir = Join-Path $gitCommonDir "agent-preflight-state"
  } else {
    $stateDir = Join-Path (Join-Path $rootDir $gitCommonDir) "agent-preflight-state"
  }
}
New-Item -ItemType Directory -Force -Path $stateDir | Out-Null

$sha256 = New-Object System.Security.Cryptography.SHA256Managed
try {
  $hashBytes = $sha256.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($stateDir))
} finally {
  $sha256.Dispose()
}
$mutexName = "Wavecrate.AgentPreflight.$(([System.BitConverter]::ToString($hashBytes)).Replace('-', ''))"
$mutex = [System.Threading.Mutex]::new($false, $mutexName)
$resultFile = Join-Path $stateDir "last-result"
$ownsMutex = $false
$isOwner = $mutex.WaitOne(0)
if ($isOwner) {
  $ownsMutex = $true
} else {
  Write-Host "[agent_preflight] another full preflight is active; waiting to coalesce."
  $abandoned = $false
  try {
    $mutex.WaitOne() | Out-Null
  } catch [System.Threading.AbandonedMutexException] {
    Write-Host "[agent_preflight] recovered an interrupted full-preflight owner."
    $abandoned = $true
  }
  try {
    if (-not $abandoned -and (Test-Path -LiteralPath $resultFile)) {
      $ownerStatus = (Get-Content -LiteralPath $resultFile -Raw).Trim()
      if ($ownerStatus -match '^\d+$') {
        Write-Host "[agent_preflight] coalesced with active full preflight (exit $ownerStatus)."
        exit ([int]$ownerStatus)
      }
    }
    Write-Host "[agent_preflight] active owner ended without a result; becoming the new owner."
    $isOwner = $true
    $ownsMutex = $true
  } finally {
    if (-not $ownsMutex) {
      $mutex.ReleaseMutex()
    }
  }
}

$exitCode = 1

$args = @(
  "-NoProfile",
  "-File",
  $checksScript
)

try {
  Write-Host "[agent_preflight] full preflight owner: $PID"
  & $psExe @args
  if ($LASTEXITCODE -ne 0) {
    throw "[agent_preflight] run_agent_ci_checks.ps1 failed with exit code $LASTEXITCODE."
  }
  $exitCode = 0
  Write-Host "[agent_preflight] OK"
} finally {
  Set-Content -LiteralPath $resultFile -Value $exitCode -NoNewline
  if ($ownsMutex) {
    $mutex.ReleaseMutex()
  }
  $mutex.Dispose()
}
