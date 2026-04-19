<#
.SYNOPSIS
Compatibility wrapper for the local CI parity lane.

.DESCRIPTION
Preserves the legacy `scripts/ci_local.ps1` entrypoint while delegating to the
canonical `scripts/ci.ps1 local` dispatcher.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "ci.ps1") local @args
exit $LASTEXITCODE
