<#
.SYNOPSIS
Compatibility wrapper for the broader integrated validation lane.

.DESCRIPTION
Preserves the legacy `scripts/ci_quick.ps1` entrypoint while delegating to the
canonical `scripts/ci.ps1 quick` dispatcher.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "ci.ps1") quick @args
exit $LASTEXITCODE
