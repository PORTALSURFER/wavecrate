Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Deletes the repo-local sandbox used by `scripts/run_sandbox.*`.

.DESCRIPTION
Removes:
- <repo>\.sandbox\sempal

Use this when sandbox state gets confusing or you want a fresh sandbox run.
#>

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$sandboxDir = Join-Path $rootDir ".sandbox\\sempal"

if (-not (Test-Path -LiteralPath $sandboxDir)) {
  Write-Host ("[clean_sandbox] nothing to remove: {0}" -f $sandboxDir)
  exit 0
}

Write-Host ("[clean_sandbox] removing: {0}" -f $sandboxDir)
Remove-Item -LiteralPath $sandboxDir -Recurse -Force -ErrorAction SilentlyContinue
Write-Host "[clean_sandbox] OK"

