Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("wavecrate-fixture-dispatch-" + [Guid]::NewGuid().ToString("N"))
$fakeBin = Join-Path $testRoot "bin"
$calls = Join-Path $testRoot "cargo-calls.txt"
New-Item -ItemType Directory -Path $fakeBin -Force | Out-Null
$oldPath = $env:PATH

try {
  $fakeCargo = Join-Path $fakeBin "cargo.cmd"
  Set-Content -LiteralPath $fakeCargo -Value @(
    "@echo off",
    'echo %*>>"%WAVECRATE_FIXTURE_TEST_CALLS%"',
    "exit /b 0"
  )
  $env:PATH = $fakeBin + [System.IO.Path]::PathSeparator + $oldPath
  $env:WAVECRATE_FIXTURE_TEST_CALLS = $calls

  & (Join-Path $rootDir "scripts/run.ps1") sandbox `
    -Dir (Join-Path $testRoot "sandbox") `
    -Fixture small-multi-source `
    -FixturePreserve `
    --log | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "sandbox fixture dispatch failed with exit code $LASTEXITCODE"
  }

  $recorded = Get-Content -LiteralPath $calls -Raw
  if ($recorded -notlike "*run --quiet --bin wavecrate-fixture -- provision --fixture small-multi-source*") {
    throw "fixture provision command was not dispatched"
  }
  if ($recorded -notlike "*--profile sandbox --no-reset*") {
    throw "fixture preserve/profile arguments were not dispatched"
  }
  if ($recorded -notlike "*run --release -- --log*") {
    throw "Wavecrate launch was not dispatched after fixture provisioning"
  }

  Clear-Content -LiteralPath $calls
  & (Join-Path $rootDir "scripts/run.ps1") sandbox `
    -Dir (Join-Path $testRoot "reset") `
    -Fixture empty | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "sandbox reset fixture dispatch failed with exit code $LASTEXITCODE"
  }
  $resetCalls = Get-Content -LiteralPath $calls
  $expectedReset = "run --quiet --bin wavecrate-fixture -- provision --fixture empty --config-base $(Join-Path $testRoot 'reset') --profile sandbox"
  if ($resetCalls -notcontains $expectedReset) {
    throw "default fixture dispatch must request a clean reset"
  }

  $invalidFailed = $false
  try {
    & (Join-Path $rootDir "scripts/run.ps1") sandbox `
      -Dir (Join-Path $testRoot "invalid") `
      -Fixture live | Out-Null
  } catch {
    $invalidFailed = $true
  }
  if (-not $invalidFailed) {
    throw "live must be rejected as a fixture name"
  }
} finally {
  $env:PATH = $oldPath
  Remove-Item Env:WAVECRATE_FIXTURE_TEST_CALLS -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "sandbox fixture dispatch checks passed"
