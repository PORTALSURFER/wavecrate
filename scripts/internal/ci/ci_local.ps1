
<#
.SYNOPSIS
Runs the PowerShell local CI core lane.

.DESCRIPTION
Runs the required GitHub CI parity lane that is practical in the PowerShell
environment. Linux-only advisory checks, perf guards, and GUI/manual lanes stay
outside this merge-blocking parity command.
#>

param(
  [switch]$SkipAgentPreflight,
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "../use_cargo_cache.ps1")

if ($Help) {
  Write-Host "Usage: scripts/ci.ps1 local [-SkipAgentPreflight]"
  Write-Host "Run the PowerShell local CI core lane used by this repository."
  Write-Host "If -SkipAgentPreflight is set, skip `scripts/internal/agent/run_agent_ci_checks.ps1`."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path

function Invoke-NativeStep {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Label,
    [Parameter(Mandatory = $true)]
    [scriptblock]$Command
  )

  & $Command
  if ($LASTEXITCODE -ne 0) {
    throw "[ci_local] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-WavecrateCargoCache
  Write-Host "[ci_local] cargo fmt --all -- --check"
  Invoke-NativeStep -Label "cargo fmt --all -- --check" -Command {
    Invoke-WavecrateCargo fmt --all -- --check
  }

  if (-not $SkipAgentPreflight) {
    Write-Host "[ci_local] scripts/internal/agent/run_agent_ci_checks.ps1"
    & (Join-Path $rootDir "scripts/internal/agent/run_agent_ci_checks.ps1")
  }

  Write-Host "[ci_local] cargo clippy --workspace --all-targets"
  Invoke-NativeStep -Label "cargo clippy --workspace --all-targets" -Command {
    Invoke-WavecrateCargo clippy --workspace --all-targets
  }

  Write-Host "[ci_local] cargo doc -p wavecrate --no-deps (RUSTDOCFLAGS=-D warnings)"
  $prevRustdocFlags = $env:RUSTDOCFLAGS
  try {
    $env:RUSTDOCFLAGS = "-D warnings"
    Invoke-NativeStep -Label "cargo doc --package wavecrate --no-deps" -Command {
      Invoke-WavecrateCargo doc --package wavecrate --no-deps
    }
  } finally {
    if ($null -eq $prevRustdocFlags) {
      Remove-Item Env:RUSTDOCFLAGS -ErrorAction SilentlyContinue
    } else {
      $env:RUSTDOCFLAGS = $prevRustdocFlags
    }
  }

  Write-Host "[ci_local] cargo nextest run --workspace --profile ci-required --all-targets --no-fail-fast"
  Invoke-NativeStep -Label "cargo nextest run --workspace --profile ci-required --all-targets --no-fail-fast" -Command {
    Invoke-WavecrateCargo nextest run --workspace --profile ci-required --all-targets --no-fail-fast
  }

  Write-Host "[ci_local] cargo test --workspace --doc"
  Invoke-NativeStep -Label "cargo test --workspace --doc" -Command {
    Invoke-WavecrateCargo test --workspace --doc
  }

  Write-Host "[ci_local] OK"
} finally {
  Pop-Location
}
