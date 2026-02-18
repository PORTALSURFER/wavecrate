Set-StrictMode -Version Latest

<#
.SYNOPSIS
Validates that MEMORY.md has a fresh update marker and updater identity.

.DESCRIPTION
This check protects agent handoff consistency by requiring a recent
`Last Updated:` timestamp and explicit `Updated By:` ownership line.
#>

$ErrorActionPreference = "Stop"

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$memoryFile = Join-Path $rootDir "MEMORY.md"
$maxAgeHours = 24
$requiredUpdater = "Codex"

if (-not (Test-Path $memoryFile)) {
  Write-Error "[memory_log] Missing required file: MEMORY.md"
  exit 1
}

$lines = Get-Content $memoryFile

$lastUpdatedLine = ($lines | Where-Object { $_ -match '^Last Updated:' } | Select-Object -First 1)
$updatedByLine = ($lines | Where-Object { $_ -match '^Updated By:' } | Select-Object -First 1)

if (-not $lastUpdatedLine) {
  Write-Error "[memory_log] MEMORY.md missing 'Last Updated:' line."
  exit 1
}

if (-not $updatedByLine) {
  Write-Error "[memory_log] MEMORY.md missing 'Updated By:' line."
  exit 1
}

$matchUpdated = [regex]::Match(
  $lastUpdatedLine,
  '^Last Updated:\s+([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z)$'
)
if (-not $matchUpdated.Success) {
  Write-Error "[memory_log] 'Last Updated:' must be ISO-8601 UTC, e.g. 2026-02-18T12:06:16Z."
  exit 1
}
$timestamp = $matchUpdated.Groups[1].Value

try {
  $updatedAt = [DateTime]::ParseExact(
    $timestamp,
    "yyyy-MM-ddTHH:mm:ssK",
    [System.Globalization.CultureInfo]::InvariantCulture,
    [System.Globalization.DateTimeStyles]::AssumeUniversal
  ).ToUniversalTime()
}
catch {
  Write-Error "[memory_log] Failed to parse timestamp in MEMORY.md: $timestamp"
  exit 1
}

$matchBy = [regex]::Match(
  $updatedByLine,
  '^Updated By:\s*(.+)$'
)
if (-not $matchBy.Success) {
  Write-Error "[memory_log] 'Updated By:' line malformed. Expected format: Updated By: Codex"
  exit 1
}
$updatedBy = $matchBy.Groups[1].Value.Trim()

if ($updatedBy -ne $requiredUpdater) {
  Write-Error "[memory_log] MEMORY.md must be updated by '$requiredUpdater'. Found: $updatedBy"
  exit 1
}

$age = (Get-Date).ToUniversalTime() - $updatedAt
if ($age.TotalHours -lt 0) {
  Write-Error "[memory_log] MEMORY.md timestamp is in the future: $timestamp"
  exit 1
}

if ($age.TotalHours -gt $maxAgeHours) {
  Write-Error "[memory_log] MEMORY.md is too stale. Last update: $timestamp ($([math]::Round($age.TotalHours))h ago)."
  exit 1
}

Write-Host "[memory_log] OK ($timestamp by $updatedBy)"
