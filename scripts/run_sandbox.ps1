<#
.SYNOPSIS
Compatibility wrapper for isolated sandbox runs.

.DESCRIPTION
Preserves the legacy `scripts/run_sandbox.ps1` entrypoint while delegating to
the canonical `scripts/run.ps1 sandbox` dispatcher.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run.ps1") sandbox @args
exit $LASTEXITCODE
