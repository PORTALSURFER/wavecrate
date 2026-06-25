<#
.SYNOPSIS
Compatibility wrapper for the main-branch guard.

.DESCRIPTION
The Wavecrate integration branch is now `main`. This wrapper remains so older
hooks or scripts that still call check_next_branch.ps1 continue to enforce the
current main-branch policy.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script = Join-Path $PSScriptRoot "check_main_branch.ps1"
$args = @()
if ($Help) {
  $args += "-Help"
}
$psExe = (Get-Process -Id $PID).Path
& $psExe -NoProfile -ExecutionPolicy Bypass -File $script @args
exit $LASTEXITCODE
