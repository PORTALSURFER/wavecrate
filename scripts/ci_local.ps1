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
  Write-Host "[ci_local] cargo fmt --all -- --check"
  cargo fmt --all -- --check

  Write-Host "[ci_local] scripts/check_migration_boundary.sh"
  $bash = Get-Command bash -ErrorAction SilentlyContinue
  if ($null -ne $bash) {
    bash ./scripts/check_migration_boundary.sh
  } else {
    Invoke-MigrationBoundaryCheckFallback -RootDir $rootDir
  }

  Write-Host "[ci_local] scripts/check_file_size_budget.ps1"
  & (Join-Path $rootDir "scripts/check_file_size_budget.ps1")

  Write-Host "[ci_local] scripts/check_manual_docs_scope.ps1"
  & (Join-Path $rootDir "scripts/check_manual_docs_scope.ps1")

  Write-Host "[ci_local] scripts/check_legacy_app_coupling.ps1"
  & (Join-Path $rootDir "scripts/check_legacy_app_coupling.ps1")

  Write-Host "[ci_local] scripts/check_rust_taste_invariants.ps1"
  & (Join-Path $rootDir "scripts/check_rust_taste_invariants.ps1")

  Write-Host "[ci_local] scripts/check_app_core_dependency_boundary.ps1"
  & (Join-Path $rootDir "scripts/check_app_core_dependency_boundary.ps1")

  Write-Host "[ci_local] cargo clippy --all-targets"
  cargo clippy --all-targets

  Write-Host "[ci_local] cargo test --all-targets"
  cargo test --all-targets

  Write-Host "[ci_local] OK"
} finally {
  Pop-Location
}
