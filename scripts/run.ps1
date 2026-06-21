<#
.SYNOPSIS
Dispatches sandbox and diagnostic run helpers.

.DESCRIPTION
Dispatches sandbox, log, and diagnostic helpers from one public entrypoint.
#>

param(
  [Parameter(Position = 0)]
  [string]$Command,
  [switch]$Help,
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$Arguments
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path
$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

if ($null -eq $Arguments) {
  $Arguments = @()
} else {
  $Arguments = @($Arguments)
}

$commands = @{
  "sandbox" = "run_sandbox.ps1"
  "clean" = "clean_sandbox.ps1"
  "logs" = "latest_log.ps1"
  "bug-bundle" = "bug_bundle.ps1"
}

if ([string]::IsNullOrWhiteSpace($Command)) {
  Write-Host "Usage: scripts/run.ps1 <sandbox|clean|logs|bug-bundle> [args...]"
  Write-Host "       scripts/run.ps1 logs <debug-overlays|debug-layout> [app args...]"
  exit 0
}

if ($Help) {
  $Arguments = @("-Help") + $Arguments
}

if (
  $Command -eq "logs" -and
  $Arguments.Count -gt 0 -and
  ($Arguments[0] -eq "debug-overlays" -or $Arguments[0] -eq "debug-layout")
) {
  $debugArg = if ($Arguments[0] -eq "debug-layout") { "--debug-layout" } else { "--debug-overlays" }
  $appArgs = @("--log", $debugArg) + @($Arguments | Select-Object -Skip 1)
  Write-Host ("[run] launching cargo run with logs and debug layout overlays: cargo run -- {0}" -f ($appArgs -join " "))
  Push-Location $rootDir
  try {
    cargo run -- @appArgs
    exit $LASTEXITCODE
  } finally {
    Pop-Location
  }
}

$scriptName = $commands[$Command]
if ($null -eq $scriptName) {
  throw "Unknown run command: $Command"
}

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "internal/run/$scriptName") @Arguments
