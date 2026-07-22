<#
.SYNOPSIS
Runs Wavecrate in an isolated sandbox config directory.

.DESCRIPTION
Runs `cargo run --release` with `WAVECRATE_CONFIG_HOME` set to an isolated sandbox
base directory plus `WAVECRATE_CONFIG_PROFILE=sandbox` so local runs (including
agent runs) do not touch real user data.

Derived paths:
- app root:  <WAVECRATE_CONFIG_HOME>\.wavecrate\profiles\sandbox
- config:    <app root>\config.toml
- logs:      <app root>\logs
#>

param(
  [string]$Dir,
  [string]$Name,
  [switch]$Temp,
  [switch]$Clean,
  [switch]$WriteDb,
  [ValidateSet("empty", "small-multi-source", "large-source")]
  [string]$Fixture,
  [switch]$FixturePreserve,
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$AppArgs
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path

function New-SandboxBase {
  param([string]$Requested, [string]$SandboxName, [bool]$UseTemp)
  if (-not [string]::IsNullOrWhiteSpace($Requested)) {
    New-Item -ItemType Directory -Path $Requested -Force | Out-Null
    return (Resolve-Path $Requested).Path
  }
  if ($UseTemp) {
    $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
    $rand = [Guid]::NewGuid().ToString("N").Substring(0, 8)
    $base = Join-Path $env:TEMP ("wavecrate-sandbox-" + $stamp + "-" + $rand)
    New-Item -ItemType Directory -Path $base -Force | Out-Null
    return (Resolve-Path $base).Path
  } else {
    if (-not [string]::IsNullOrWhiteSpace($SandboxName)) {
      $base = Join-Path $rootDir (".sandbox/wavecrate/" + $SandboxName)
    } else {
      $base = Join-Path $rootDir ".sandbox/wavecrate"
    }
    New-Item -ItemType Directory -Path $base -Force | Out-Null
    return (Resolve-Path $base).Path
  }
}

if ($Temp -and -not [string]::IsNullOrWhiteSpace($Name)) {
  throw "[run_sandbox][error] -Temp and -Name are mutually exclusive."
}
if (-not [string]::IsNullOrWhiteSpace($Dir) -and -not [string]::IsNullOrWhiteSpace($Name)) {
  throw "[run_sandbox][error] -Dir and -Name are mutually exclusive."
}
if ($FixturePreserve -and [string]::IsNullOrWhiteSpace($Fixture)) {
  throw "[run_sandbox][error] -FixturePreserve requires -Fixture <name>."
}

$sandboxBase = New-SandboxBase -Requested $Dir -SandboxName $Name -UseTemp ([bool]$Temp)
$env:WAVECRATE_CONFIG_HOME = $sandboxBase
$env:WAVECRATE_CONFIG_PROFILE = "sandbox"

if (-not [string]::IsNullOrWhiteSpace($Fixture)) {
  $WriteDb = $true
  Remove-Item Env:WAVECRATE_SOURCE_DB_READ_ONLY -ErrorAction SilentlyContinue
  $fixtureArgs = @(
    "run", "--quiet", "--bin", "wavecrate-fixture", "--",
    "provision", "--fixture", $Fixture, "--config-base", $sandboxBase,
    "--profile", "sandbox"
  )
  if ($FixturePreserve) {
    $fixtureArgs += "--no-reset"
  }
  Push-Location $rootDir
  try {
    & cargo @fixtureArgs | Out-Null
    $fixtureStatus = $LASTEXITCODE
  } finally {
    Pop-Location
  }
  if ($fixtureStatus -ne 0) {
    if ($Temp -or $Clean) {
      Remove-Item -LiteralPath $sandboxBase -Recurse -Force -ErrorAction SilentlyContinue
    }
    throw "fixture provisioning failed with exit code $fixtureStatus"
  }
  Write-Host ("[run_sandbox] fixture={0}" -f $Fixture)
  Write-Host ("[run_sandbox] fixture_manifest={0}" -f (Join-Path $sandboxBase ".wavecrate/fixtures/$Fixture/fixture-manifest.json"))
}

if ($WriteDb) {
  Remove-Item Env:WAVECRATE_SOURCE_DB_READ_ONLY -ErrorAction SilentlyContinue
} else {
  $env:WAVECRATE_SOURCE_DB_READ_ONLY = "1"
}

$appRoot = Join-Path $sandboxBase ".wavecrate\\profiles\\sandbox"
$configPath = Join-Path $appRoot "config.toml"
$logsDir = Join-Path $appRoot "logs"

Write-Host ("[run_sandbox] repo_root={0}" -f $rootDir)
Write-Host ("[run_sandbox] WAVECRATE_CONFIG_HOME={0}" -f $sandboxBase)
Write-Host ("[run_sandbox] WAVECRATE_CONFIG_PROFILE={0}" -f $env:WAVECRATE_CONFIG_PROFILE)
Write-Host ("[run_sandbox] app_root={0}" -f $appRoot)
Write-Host ("[run_sandbox] config={0}" -f $configPath)
Write-Host ("[run_sandbox] logs={0}" -f $logsDir)
Write-Host "[run_sandbox] CONTRACT: app config/logs will NOT be read/written from your real user profile dirs (it uses WAVECRATE_CONFIG_HOME)."
if ($WriteDb) {
  Write-Host "[run_sandbox] Source DB mode: write-enabled (explicit override)."
} else {
  Write-Host "[run_sandbox] Source DB mode: read-only (default for agent safety)."
}

if (-not $WriteDb) {
  Write-Host "[run_sandbox] DB writes to source trees are blocked by default."
} else {
  Write-Host "[run_sandbox] DB writes to source trees are enabled for this run."
}

Write-Host "[run_sandbox] Can still write:"
Write-Host ("[run_sandbox]   - sandbox dir: {0}" -f $sandboxBase)
Write-Host ("[run_sandbox]   - cargo build artifacts: {0} (and your rustup/cargo caches)" -f (Join-Path $rootDir "target"))
if ($Temp) {
  Write-Host "[run_sandbox] Ephemeral mode: sandbox dir will be deleted on exit."
}

Push-Location $rootDir
try {
  $runStatus = 0
  try {
    cargo run --release -- @AppArgs
    $runStatus = $LASTEXITCODE
    if ($runStatus -ne 0) {
      Write-Host "[run_sandbox][warn] cargo run failed with exit code $runStatus."
    }
  } catch {
    if ($LASTEXITCODE -ne 0) {
      $runStatus = $LASTEXITCODE
    } else {
      $runStatus = 1
    }
    Write-Host "[run_sandbox][warn] cargo run failed with exit code $runStatus."
  }
} finally {
  Pop-Location
  if ($Temp -or $Clean) {
    Remove-Item -LiteralPath $sandboxBase -Recurse -Force -ErrorAction SilentlyContinue
  }
}

exit $runStatus
