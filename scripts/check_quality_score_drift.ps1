Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Checks quality score drift for high-visibility guardrails.

.DESCRIPTION
Runs `scripts/check_quality_score_drift.sh` with arguments and surfaces failures to
PowerShell callers.
#>

param(
  [string]$Base = "",
  [string]$Head = "HEAD"
)

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$scriptPath = Join-Path $rootDir "scripts/check_quality_score_drift.sh"

if (-not (Get-Command bash -ErrorAction SilentlyContinue)) {
  throw "Bash is required to run scripts/check_quality_score_drift.sh"
}

$args = @()
if ($Base) {
  $args += "--base", $Base
}
if ($Head) {
  $args += "--head", $Head
}

& bash "$scriptPath" @args
