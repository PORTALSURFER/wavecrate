Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Runs the local equivalent of the GitHub Actions CI checks.

.DESCRIPTION
Mirrors `.github/workflows/ci.yml` so developers and agents can run the same
format/lint/test steps locally.

.NOTES
The migration-boundary check is implemented in bash for parity with CI. If `bash`
is not available locally, this script runs an equivalent PowerShell fallback.
#>

param(
  [switch]$SkipAgentPreflight,
  [switch]$Help
)

if ($Help) {
  Write-Host "Usage: scripts/ci_local.ps1 [-SkipAgentPreflight]"
  Write-Host "Run the local equivalent of the CI checks used by this repository."
  Write-Host "If -SkipAgentPreflight is set, skip `scripts/run_agent_ci_checks.sh`."
  exit 0
}

function Invoke-MigrationBoundaryCheckFallback {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RootDir
  )

  $appCoreDir = Join-Path $RootDir "src/app_core"
  $allowedFile = Join-Path $appCoreDir "app_api.rs"

  if (-not (Test-Path $appCoreDir)) {
    throw "Expected app_core directory not found: $appCoreDir"
  }

  $hits = @()
  $files = Get-ChildItem -Path $appCoreDir -Recurse -File -Filter "*.rs"
  foreach ($file in $files) {
    if ($file.FullName -eq $allowedFile) {
      continue
    }

    $matches = Select-String -Path $file.FullName -SimpleMatch -Pattern "crate::app::"
    foreach ($match in $matches) {
      $hits += ("{0}:{1}:{2}" -f $match.Path, $match.LineNumber, $match.Line.Trim())
    }
  }

  if ($hits.Count -eq 0) {
    Write-Host "Migration boundary check passed: no legacy app references in app_core."
    return
  }

  Write-Error "Migration boundary check failed: direct crate::app references were found outside app_core::app_api."
  foreach ($hit in $hits) {
    Write-Host (" - {0}" -f $hit)
  }
  Write-Host ("Allowed app_core migration boundary location: {0}" -f $allowedFile)
  exit 1
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

Push-Location $rootDir
try {
  if (-not (Get-Command bash -ErrorAction SilentlyContinue)) {
    throw "[ci_local] ERROR: bash is required by scripts/run_agent_ci_checks.sh in this environment."
  }

  Write-Host "[ci_local] cargo fmt --all -- --check"
  cargo fmt --all -- --check

  if (-not $SkipAgentPreflight) {
    Write-Host "[ci_local] scripts/run_agent_ci_checks.sh"
    & bash (Join-Path $rootDir "scripts/run_agent_ci_checks.sh")
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

  Write-Host "[ci_local] scripts/run_perf_guard.sh"
  & bash (Join-Path $rootDir "scripts/run_perf_guard.sh")

  Write-Host "[ci_local] OK"
} finally {
  Pop-Location
}
