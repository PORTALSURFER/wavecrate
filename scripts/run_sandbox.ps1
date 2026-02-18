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
  [switch]$Clean
)

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function New-SandboxBase {
  param([string]$Requested)
  if (-not [string]::IsNullOrWhiteSpace($Requested)) {
    New-Item -ItemType Directory -Path $Requested -Force | Out-Null
    return (Resolve-Path $Requested).Path
  }
  $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
  $base = Join-Path $env:TEMP ("sempal-sandbox-" + $stamp)
  New-Item -ItemType Directory -Path $base -Force | Out-Null
  return $base
}

$sandboxBase = New-SandboxBase -Requested $Dir
$env:SEMPAL_CONFIG_HOME = $sandboxBase

$appRoot = Join-Path $sandboxBase ".sempal"
$configPath = Join-Path $appRoot "config.toml"
$logsDir = Join-Path $appRoot "logs"

Write-Host ("[run_sandbox] repo_root={0}" -f $rootDir)
Write-Host ("[run_sandbox] SEMPAL_CONFIG_HOME={0}" -f $sandboxBase)
Write-Host ("[run_sandbox] app_root={0}" -f $appRoot)
Write-Host ("[run_sandbox] config={0}" -f $configPath)
Write-Host ("[run_sandbox] logs={0}" -f $logsDir)
Write-Host "[run_sandbox] NOTE: this run uses an isolated config/log directory."

Push-Location $rootDir
try {
  cargo run --release -- $args
} finally {
  Pop-Location
  if ($Clean) {
    Remove-Item -LiteralPath $sandboxBase -Recurse -Force -ErrorAction SilentlyContinue
  }
}

