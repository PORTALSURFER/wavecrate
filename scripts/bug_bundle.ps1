<#
.SYNOPSIS
Compatibility wrapper for the bug-bundle helper.

.DESCRIPTION
Preserves the legacy `scripts/bug_bundle.ps1` entrypoint while delegating to
the canonical `scripts/run.ps1 bug-bundle` dispatcher.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path

& $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run.ps1") bug-bundle @args
exit $LASTEXITCODE
