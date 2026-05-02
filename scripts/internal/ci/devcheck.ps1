<#
.SYNOPSIS
Runs the fastest local compile/smoke check.

.DESCRIPTION
Executes the smallest useful development gate by type-checking library, test,
and binary targets without running the test suite. Use this during normal edit
loops, then escalate to `scripts/ci.ps1 quick` before commit and
`scripts/ci.ps1 local` for full CI parity.
#>

param(
  [switch]$AppOnly,
  [switch]$Workspace,
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "../use_cargo_cache.ps1")

if ($Help) {
  Write-Host "Usage: scripts/ci.ps1 smoke [-AppOnly] [-Workspace]"
  Write-Host "Run the compile/smoke gate."
  Write-Host "Use -AppOnly for the lightest app-only check."
  Write-Host "Use -Workspace for the broad workspace compile gate."
  Write-Host "For fast test coverage, use `scripts/ci.ps1 quick`."
  exit 0
}

if ($AppOnly -and $Workspace) {
  throw "AppOnly and Workspace are mutually exclusive."
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
    throw "[devcheck] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-SempalCargoCache
  Write-Host "[devcheck] branch policy"
  Invoke-NativeStep -Label "branch policy" -Command {
    & (Join-Path $rootDir "scripts/check.ps1") next-branch
  }

  Write-Host "[devcheck] cargo check --manifest-path vendor/radiant/Cargo.toml"
  Invoke-NativeStep -Label "cargo check --manifest-path vendor/radiant/Cargo.toml" -Command {
    Invoke-SempalCargo check --manifest-path vendor/radiant/Cargo.toml
  }

  Write-Host "[devcheck] cargo check --manifest-path vendor/radiant/Cargo.toml --example generic_native --no-default-features"
  Invoke-NativeStep -Label "cargo check --manifest-path vendor/radiant/Cargo.toml --example generic_native --no-default-features" -Command {
    Invoke-SempalCargo check --manifest-path vendor/radiant/Cargo.toml --example generic_native --no-default-features
  }

  if ($AppOnly) {
    Write-Host "[devcheck] cargo check -p sempal --lib --bin sempal"
    Invoke-NativeStep -Label "cargo check -p sempal --lib --bin sempal" -Command {
      Invoke-SempalCargo check -p sempal --lib --bin sempal
    }
  } elseif ($Workspace) {
    Write-Host "[devcheck] cargo check --workspace --tests --bins"
    Invoke-NativeStep -Label "cargo check --workspace --tests --bins" -Command {
      Invoke-SempalCargo check --workspace --tests --bins
    }
  } else {
    Write-Host "[devcheck] cargo check -p sempal --tests --bins"
    Invoke-NativeStep -Label "cargo check -p sempal --tests --bins" -Command {
      Invoke-SempalCargo check -p sempal --tests --bins
    }
  }

  Write-Host "[devcheck] OK"
} finally {
  Pop-Location
}
