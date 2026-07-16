
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

$lockDir = Join-Path $stateDir "run.lock"
$ownsLock = $false
$resultFile = $null

while (-not $ownsLock) {
  try {
    New-Item -ItemType Directory -Path $lockDir -ErrorAction Stop | Out-Null
    $ownsLock = $true
  } catch [System.IO.IOException] {
    $ownerPid = $null
    $ownerResult = $null
    $ownerFile = Join-Path $lockDir "owner"
    if (Test-Path -LiteralPath $ownerFile) {
      $ownerFields = (Get-Content -LiteralPath $ownerFile -Raw).Trim() -split "`t", 2
      if ($ownerFields.Count -eq 2) {
        $ownerPid = $ownerFields[0]
        $ownerResult = $ownerFields[1]
      }
    }

    $ownerRunning = $false
    if ($null -ne $ownerPid -and $ownerPid -match '^\d+$') {
      $ownerRunning = $null -ne (Get-Process -Id ([int]$ownerPid) -ErrorAction SilentlyContinue)
    }
    if ($ownerRunning) {
      Write-Host "[agent_preflight] another full preflight is active (pid $ownerPid); waiting to coalesce."
      while ((Test-Path -LiteralPath $lockDir) -and (Get-Process -Id ([int]$ownerPid) -ErrorAction SilentlyContinue)) {
        Start-Sleep -Milliseconds 100
      }
      if ($null -ne $ownerResult -and (Test-Path -LiteralPath $ownerResult)) {
        $ownerStatus = (Get-Content -LiteralPath $ownerResult -Raw).Trim()
        if ($ownerStatus -match '^\d+$') {
          Write-Host "[agent_preflight] coalesced with active full preflight (exit $ownerStatus)."
          exit ([int]$ownerStatus)
        }
      }
      Write-Host "[agent_preflight] active owner ended without a result; retrying ownership."
    } else {
      Write-Host "[agent_preflight] clearing stale single-flight state."
      Remove-Item -LiteralPath $lockDir -Force -Recurse -ErrorAction SilentlyContinue
    }
  }
}

$resultFile = Join-Path $stateDir ("result.{0}.{1}" -f $PID, [guid]::NewGuid().ToString("N"))
Set-Content -LiteralPath (Join-Path $lockDir "owner") -Value ("{0}`t{1}" -f $PID, $resultFile) -NoNewline
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
  Remove-Item -LiteralPath $lockDir -Force -Recurse -ErrorAction SilentlyContinue
}
