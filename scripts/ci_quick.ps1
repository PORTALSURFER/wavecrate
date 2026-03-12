<#
.SYNOPSIS
Runs a fast local development test pass.

.DESCRIPTION
Executes the everyday fast test loop for normal development by running the
quick nextest profile over library and integration tests. This intentionally
skips support-tool binaries, slower recovery tests, clippy, rustdoc, benches,
perf guards, and other CI-parity checks reserved for `scripts/ci_local.ps1`.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "use_cargo_cache.ps1")

if ($Help) {
  Write-Host "Usage: scripts/ci_quick.ps1"
  Write-Host "Run the fast local development test loop."
  Write-Host "For the compile-only smoke gate, use `scripts/devcheck.ps1`."
  Write-Host "For full CI parity, use `scripts/ci_local.ps1`."
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
    throw "[ci_quick] Step failed ($Label) with exit code $LASTEXITCODE."
  }
}

Push-Location $rootDir
try {
  Enable-SempalCargoCache
  Write-Host "[ci_quick] branch policy"
  Invoke-NativeStep -Label "branch policy" -Command {
    & (Join-Path $PSScriptRoot "check_next_branch.ps1")
  }

  Write-Host "[ci_quick] cargo nextest run -p sempal --profile quick --lib --tests"
  Invoke-NativeStep -Label "cargo nextest run -p sempal --profile quick --lib --tests" -Command {
    cargo nextest run -p sempal --profile quick --lib --tests
  }

  Write-Host "[ci_quick] OK"
} finally {
  Pop-Location
}
