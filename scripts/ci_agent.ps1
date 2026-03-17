<#
.SYNOPSIS
Runs the agent-safe local development validation loop.

.DESCRIPTION
This lane avoids `cargo-nextest` and the broader GUI contract/integration
wrappers so it can run in constrained Windows environments where Application
Control blocks the `cargo-nextest.exe` binary. It keeps the edit loop grounded
by running the normal compile smoke gate plus the full sempal library test
suite in one cargo process.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "use_cargo_cache.ps1")

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

if ($Help) {
  Write-Host "Usage: scripts/ci_agent.ps1"
  Write-Host "Run the agent-safe local validation loop without cargo-nextest."
  Write-Host "For the broader integrated lane, use `scripts/ci_quick.ps1`."
  Write-Host "For full CI parity, use `scripts/ci_local.ps1`."
  exit 0
}

function Invoke-NativeStep {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Label,
    [Parameter(Mandatory = $true)]
    [scriptblock]$Command
  )

  & $Command
  if ($LASTEXITCODE -ne 0) {
    throw "[ci_agent] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-SempalCargoCache
  Write-Host "[ci_agent] branch policy"
  Invoke-NativeStep -Label "branch policy" -Command {
    & (Join-Path $PSScriptRoot "check_next_branch.ps1")
  }

  Write-Host "[ci_agent] scripts/devcheck.ps1"
  & (Join-Path $PSScriptRoot "devcheck.ps1")

  Write-Host "[ci_agent] cargo test -p sempal --lib -- --test-threads=1"
  Invoke-NativeStep -Label "cargo test -p sempal --lib -- --test-threads=1" -Command {
    cargo test -p sempal --lib -- --test-threads=1
  }

  Write-Host "[ci_agent] OK"
} finally {
  Pop-Location
}
