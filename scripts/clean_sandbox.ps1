<#
.SYNOPSIS
Compatibility wrapper for sandbox cleanup.

.DESCRIPTION
Preserves the legacy `scripts/clean_sandbox.ps1` entrypoint while delegating to
the canonical `scripts/run.ps1 clean` dispatcher.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run.ps1") clean @args
exit $LASTEXITCODE
