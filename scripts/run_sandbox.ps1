Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Runs Sempal in an isolated sandbox config directory.

.DESCRIPTION
Runs `cargo run --release` with `SEMPAL_CONFIG_HOME` set to an isolated sandbox
base directory so local runs (including agent runs) do not touch real user data.

Derived paths:
- app root:  <SEMPAL_CONFIG_HOME>\.sempal
- config:    <app root>\config.toml
- logs:      <app root>\logs
#>

param(
  [string]$Dir,
  [string]$Name,
  [switch]$Temp,
  [switch]$Clean,
  [switch]$WriteDb,
  [switch]$AllowUserLibraryDbWrite
)

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function New-SandboxBase {
  param([string]$Requested, [string]$SandboxName, [bool]$UseTemp)
  if (-not [string]::IsNullOrWhiteSpace($Requested)) {
    New-Item -ItemType Directory -Path $Requested -Force | Out-Null
    return (Resolve-Path $Requested).Path
  }
  if ($UseTemp) {
    $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
    $rand = [Guid]::NewGuid().ToString("N").Substring(0, 8)
    $base = Join-Path $env:TEMP ("sempal-sandbox-" + $stamp + "-" + $rand)
    New-Item -ItemType Directory -Path $base -Force | Out-Null
    return (Resolve-Path $base).Path
  } else {
    if (-not [string]::IsNullOrWhiteSpace($SandboxName)) {
      $base = Join-Path $rootDir (".sandbox/sempal/" + $SandboxName)
    } else {
      $base = Join-Path $rootDir ".sandbox/sempal"
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

if ($WriteDb) {
  Remove-Item Env:SEMPAL_SOURCE_DB_READ_ONLY -ErrorAction SilentlyContinue
} else {
  $env:SEMPAL_SOURCE_DB_READ_ONLY = "1"
}
if ($AllowUserLibraryDbWrite) {
  $env:SEMPAL_ALLOW_USER_LIBRARY_DB_WRITE = "1"
} else {
  Remove-Item Env:SEMPAL_ALLOW_USER_LIBRARY_DB_WRITE -ErrorAction SilentlyContinue
}

$sandboxBase = New-SandboxBase -Requested $Dir -SandboxName $Name -UseTemp ([bool]$Temp)
$env:SEMPAL_CONFIG_HOME = $sandboxBase

$appRoot = Join-Path $sandboxBase ".sempal"
$configPath = Join-Path $appRoot "config.toml"
$logsDir = Join-Path $appRoot "logs"

Write-Host ("[run_sandbox] repo_root={0}" -f $rootDir)
Write-Host ("[run_sandbox] SEMPAL_CONFIG_HOME={0}" -f $sandboxBase)
Write-Host ("[run_sandbox] app_root={0}" -f $appRoot)
Write-Host ("[run_sandbox] config={0}" -f $configPath)
Write-Host ("[run_sandbox] logs={0}" -f $logsDir)
Write-Host "[run_sandbox] CONTRACT: app config/logs will NOT be read/written from your real user profile dirs (it uses SEMPAL_CONFIG_HOME)."
if ($WriteDb) {
  Write-Host "[run_sandbox] Source DB mode: write-enabled (explicit override)."
} else {
  Write-Host "[run_sandbox] Source DB mode: read-only (default for agent safety)."
}
if ($AllowUserLibraryDbWrite) {
  Write-Host "[run_sandbox] User-library DB writes: explicitly allowed."
} else {
  Write-Host "[run_sandbox] User-library DB writes: blocked."
}

if (-not $WriteDb) {
  Write-Host "[run_sandbox] DB writes to source trees are blocked by default."
} else {
  Write-Host "[run_sandbox] DB writes to source trees are enabled for this run."
  if (-not $AllowUserLibraryDbWrite) {
    Write-Host "[run_sandbox] User-library-like source roots are still blocked unless -AllowUserLibraryDbWrite is set."
  }
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
    cargo run --release -- $args
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

if (Test-Path $logsDir) {
  $contractsDir = Join-Path $logsDir "contracts"
  if (Test-Path $contractsDir) {
    $latestContract = Get-ChildItem -Path $contractsDir -Filter "run_contract_*.ndjson" |
      Sort-Object LastWriteTime -Descending |
      Select-Object -First 1

    if ($null -ne $latestContract) {
      $latestContractPath = $latestContract.FullName
      $latestManifestPath = $latestContractPath -replace "run_contract_", "run_manifest_"
      $latestManifestPath = [System.IO.Path]::ChangeExtension($latestManifestPath, ".json")

      Write-Host "[run_sandbox] latest run_contract=$latestContractPath"
      Write-Host "[run_sandbox] latest run_manifest=$latestManifestPath"

      if (Test-Path $latestManifestPath) {
        try {
          $manifest = Get-Content -Raw -LiteralPath $latestManifestPath | ConvertFrom-Json
          $finalStatus = if ($manifest.PSObject.Properties["exit_status"]) { $manifest.exit_status } else { "<missing>" }
          Write-Host "[run_sandbox] run outcome=$finalStatus"
        } catch {
          Write-Host "[run_sandbox][warn] Failed to parse run manifest: $latestManifestPath"
        }
      } else {
        Write-Host "[run_sandbox] run manifest missing or not written yet: $latestManifestPath"
      }
    }
  }
}

exit $runStatus
