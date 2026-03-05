
<#
.SYNOPSIS
Refreshes MEMORY.md with a UTC timestamp and updater identity.

.DESCRIPTION
Updates (or appends) `Last Updated:` and `Updated By:` lines in `MEMORY.md`.
#>

param(
  [string]$Updater = "Codex",
  [string]$Timestamp = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


if ([string]::IsNullOrWhiteSpace($Updater)) {
  throw "[memory_refresh] Updater must be non-empty."
}

if ([string]::IsNullOrWhiteSpace($Timestamp)) {
  $Timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$memoryFile = Join-Path $rootDir "MEMORY.md"

if (-not (Test-Path -LiteralPath $memoryFile)) {
  throw "[memory_refresh] Missing required file: MEMORY.md"
}

$lines = Get-Content -LiteralPath $memoryFile
$updatedLines = New-Object System.Collections.Generic.List[string]
$foundTimestamp = $false
$foundUpdater = $false

foreach ($line in $lines) {
  if ($line.StartsWith("Last Updated:")) {
    $updatedLines.Add("Last Updated: $Timestamp")
    $foundTimestamp = $true
    continue
  }
  if ($line.StartsWith("Updated By:")) {
    $updatedLines.Add("Updated By: $Updater")
    $foundUpdater = $true
    continue
  }
  $updatedLines.Add($line)
}

if (-not $foundTimestamp) {
  $updatedLines.Add("Last Updated: $Timestamp")
}
if (-not $foundUpdater) {
  $updatedLines.Add("Updated By: $Updater")
}

$content = ($updatedLines -join "`n") + "`n"
Set-Content -LiteralPath $memoryFile -Value $content -Encoding UTF8

Write-Host "[memory_refresh] Updated MEMORY.md (Last Updated: $Timestamp, Updated By: $Updater)"
