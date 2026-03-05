
<#
.SYNOPSIS
Runs the local equivalent of the GitHub Actions CI checks.

.DESCRIPTION
Mirrors `.github/workflows/ci.yml` so developers and agents can run the same
format/lint/test steps locally.
#>

param(
  [switch]$SkipAgentPreflight,
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


if ($Help) {
  Write-Host "Usage: scripts/ci_local.ps1 [-SkipAgentPreflight]"
  Write-Host "Run the local equivalent of the CI checks used by this repository."
  Write-Host "If -SkipAgentPreflight is set, skip `scripts/run_agent_ci_checks.ps1`."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

Push-Location $rootDir
try {
  Write-Host "[ci_local] cargo fmt --all -- --check"
  cargo fmt --all -- --check

  if (-not $SkipAgentPreflight) {
    Write-Host "[ci_local] scripts/run_agent_ci_checks.ps1"
    & (Join-Path $rootDir "scripts/run_agent_ci_checks.ps1")
  }

  Write-Host "[ci_local] cargo clippy --all-targets"
  cargo clippy --all-targets

  Write-Host "[ci_local] cargo doc -p sempal --no-deps (RUSTDOCFLAGS=-D warnings)"
  $prevRustdocFlags = $env:RUSTDOCFLAGS
  try {
    $env:RUSTDOCFLAGS = "-D warnings"
    cargo doc -p sempal --no-deps
  } finally {
    if ($null -eq $prevRustdocFlags) {
      Remove-Item Env:RUSTDOCFLAGS -ErrorAction SilentlyContinue
    } else {
      $env:RUSTDOCFLAGS = $prevRustdocFlags
    }
  }

  Write-Host "[ci_local] cargo test --all-targets"
  cargo test --all-targets

  Write-Host "[ci_local] scripts/run_perf_guard.ps1"
  & (Join-Path $rootDir "scripts/run_perf_guard.ps1")

  Write-Host "[ci_local] OK"
} finally {
  Pop-Location
}
