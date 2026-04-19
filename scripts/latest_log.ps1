<#
.SYNOPSIS
Compatibility wrapper for the latest-log helper.

.DESCRIPTION
Preserves the legacy `scripts/latest_log.ps1` entrypoint while delegating to
the canonical `scripts/run.ps1 logs` dispatcher.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run.ps1") logs @args
exit $LASTEXITCODE
