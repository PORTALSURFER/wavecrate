<#
.SYNOPSIS
Runs a broader integrated local development test pass.

.DESCRIPTION
Executes the broader integrated local test loop by running the quick nextest
profile over library and integration tests plus the semantic GUI contract lane.
This intentionally skips support-tool binaries, slower recovery tests, clippy,
rustdoc, benches, perf guards, and other CI-parity checks
reserved for `scripts/ci.ps1 local`.
#>

param(
  [switch]$Workspace,
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "../use_cargo_cache.ps1")

if ($Help) {
  Write-Host "Usage: scripts/ci.ps1 quick [-Workspace]"
  Write-Host "Run the broader integrated local development test loop."
  Write-Host "Use -Workspace for the full workspace nextest lane."
  Write-Host "For the constrained agent-safe lane, use `scripts/ci.ps1 agent`."
  Write-Host "For the compile-only smoke gate, use `scripts/ci.ps1 smoke`."
  Write-Host "For full CI parity, use `scripts/ci.ps1 local`."
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
    throw "[ci_quick] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-WavecrateCargoCache
  Write-Host "[ci_quick] branch policy"
  Invoke-NativeStep -Label "branch policy" -Command {
    & (Join-Path $rootDir "scripts/check.ps1") main-branch
  }

  if ($Workspace) {
    Write-Host "[ci_quick] cargo nextest run --workspace --profile quick --all-targets"
    Invoke-NativeStep -Label "cargo nextest run --workspace --profile quick --all-targets" -Command {
      Invoke-WavecrateCargo nextest run --workspace --profile quick --all-targets
    }
  } else {
    Write-Host "[ci_quick] cargo nextest run --package wavecrate --profile quick --lib --tests"
    Invoke-NativeStep -Label "cargo nextest run --package wavecrate --profile quick --lib --tests" -Command {
      Invoke-WavecrateCargo nextest run --package wavecrate --profile quick --lib --tests
    }
  }

  Write-Host "[ci_quick] scripts/gui.ps1 contract"
  Invoke-NativeStep -Label "scripts/gui.ps1 contract" -Command {
    & (Join-Path $rootDir "scripts/gui.ps1") contract
  }

  Write-Host "[ci_quick] OK"
} finally {
  Pop-Location
}
