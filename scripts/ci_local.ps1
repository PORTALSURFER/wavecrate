
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

. (Join-Path $PSScriptRoot "use_cargo_cache.ps1")

if ($Help) {
  Write-Host "Usage: scripts/ci_local.ps1 [-SkipAgentPreflight]"
  Write-Host "Run the local equivalent of the CI checks used by this repository."
  Write-Host "If -SkipAgentPreflight is set, skip `scripts/run_agent_ci_checks.ps1`."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

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
  Enable-SempalCargoCache
  Write-Host "[ci_local] cargo fmt --all -- --check"
  Invoke-NativeStep -Label "cargo fmt --all -- --check" -Command {
    Invoke-SempalCargo fmt --all -- --check
  }

  if (-not $SkipAgentPreflight) {
    Write-Host "[ci_local] scripts/run_agent_ci_checks.ps1"
    & (Join-Path $rootDir "scripts/run_agent_ci_checks.ps1")
  }

  Write-Host "[ci_local] cargo clippy --workspace --all-targets"
  Invoke-NativeStep -Label "cargo clippy --workspace --all-targets" -Command {
    Invoke-SempalCargo clippy --workspace --all-targets
  }

  Write-Host "[ci_local] cargo doc -p sempal --no-deps (RUSTDOCFLAGS=-D warnings)"
  $prevRustdocFlags = $env:RUSTDOCFLAGS
  try {
    $env:RUSTDOCFLAGS = "-D warnings"
    Invoke-NativeStep -Label "cargo doc -p sempal --no-deps" -Command {
      Invoke-SempalCargo doc -p sempal --no-deps
    }
  } finally {
    if ($null -eq $prevRustdocFlags) {
      Remove-Item Env:RUSTDOCFLAGS -ErrorAction SilentlyContinue
    } else {
      $env:RUSTDOCFLAGS = $prevRustdocFlags
    }
  }

  Write-Host "[ci_local] cargo nextest run --workspace --all-targets --no-fail-fast"
  Invoke-NativeStep -Label "cargo nextest run --workspace --all-targets --no-fail-fast" -Command {
    Invoke-SempalCargo nextest run --workspace --all-targets --no-fail-fast
  }

  Write-Host "[ci_local] cargo test --workspace --doc"
  Invoke-NativeStep -Label "cargo test --workspace --doc" -Command {
    Invoke-SempalCargo test --workspace --doc
  }

  Write-Host "[ci_local] scripts/run_perf_guard.ps1"
  & (Join-Path $rootDir "scripts/run_perf_guard.ps1")

  Write-Host "[ci_local] OK"
} finally {
  Pop-Location
}
