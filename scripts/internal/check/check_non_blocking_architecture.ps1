<#
.SYNOPSIS
Runs the required non-blocking app architecture guardrails.

.DESCRIPTION
This check pins the Radiant and Wavecrate tests that enforce the locked-down
app-facing runtime contract. It is intentionally deterministic: static
guardrails and a controlled strict-diagnostics harness are required, while
flaky timing benchmarks remain outside this lane.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "../use_cargo_cache.ps1")

if ($Help) {
  Write-Host "Usage: scripts/internal/check/check_non_blocking_architecture.ps1"
  Write-Host ""
  Write-Host "Run required Radiant and Wavecrate non-blocking architecture guardrails."
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

  Write-Host ("[non_blocking_architecture] {0}" -f $Label)
  & $Command
  if ($LASTEXITCODE -ne 0) {
    throw "[non_blocking_architecture] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-WavecrateCargoCache

  Invoke-NativeStep -Label "Radiant synthetic blocking-token fixture" -Command {
    Invoke-WavecrateCargo @("test", "--manifest-path", "vendor/radiant/Cargo.toml", "guardrail_reports_file_line_and_guidance_for_blocking_tokens")
  }

  Invoke-NativeStep -Label "Radiant app/runtime/example guardrails" -Command {
    Invoke-WavecrateCargo @("test", "--manifest-path", "vendor/radiant/Cargo.toml", "--test", "generic_surface_guardrails", "source_quality::runtime::commands_and_app")
  }

  Invoke-NativeStep -Label "Wavecrate app-facing blocking guardrail" -Command {
    Invoke-WavecrateCargo @("test", "-p", "wavecrate", "--no-default-features", "native_app_ui_update_paths_do_not_call_blocking_business_apis")
  }

  Invoke-NativeStep -Label "Wavecrate strict slow-handler diagnostics harness" -Command {
    Invoke-WavecrateCargo @("test", "-p", "wavecrate", "--no-default-features", "rapid_navigation_harness_keeps_ui_responsive_while_business_work_is_slow")
  }

  Write-Host "[non_blocking_architecture] OK"
  exit 0
} finally {
  Pop-Location
}
