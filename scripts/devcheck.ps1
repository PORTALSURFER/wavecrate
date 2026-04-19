<#
.SYNOPSIS
Compatibility wrapper for the smoke validation lane.

.DESCRIPTION
Preserves the legacy `scripts/devcheck.ps1` entrypoint while delegating to the
canonical `scripts/ci.ps1 smoke` dispatcher.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "ci.ps1") smoke @args
exit $LASTEXITCODE
