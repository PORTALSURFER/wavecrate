<#
.SYNOPSIS
Runs the GUI automation and contract validation lane.

.DESCRIPTION
Executes the GUI action-catalog tests, GUI fixture/automation tests, and the
Radiant toolbar hit-test smoke. The lane uses the shared Cargo wrapper helper
so inherited `sccache` or temp-directory issues degrade cleanly to direct
`rustc` instead of failing before the test commands run.
#>

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "use_cargo_cache.ps1")

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
    throw "[gui-contract] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-SempalCargoCache

  Write-Host "[gui-contract] cargo test app_core::actions::tests"
  Invoke-NativeStep -Label "cargo test app_core::actions::tests" -Command {
    Invoke-SempalCargo test app_core::actions::tests
  }

  Write-Host "[gui-contract] cargo test gui_test::"
  Invoke-NativeStep -Label "cargo test gui_test::" -Command {
    Invoke-SempalCargo test gui_test::
  }

  Write-Host "[gui-contract] cargo test --manifest-path vendor/radiant/Cargo.toml toolbar_hit_test_focuses_browser_search"
  Invoke-NativeStep -Label "cargo test --manifest-path vendor/radiant/Cargo.toml toolbar_hit_test_focuses_browser_search" -Command {
    Invoke-SempalCargo test --manifest-path vendor/radiant/Cargo.toml toolbar_hit_test_focuses_browser_search
  }
} finally {
  Pop-Location
}
