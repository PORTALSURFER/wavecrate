<#
.SYNOPSIS
Reports line counts for files in the file-size budget allowlist.

.DESCRIPTION
PowerShell equivalent of `report_file_size_budget_allowlist.sh`.

This is intended for scheduled entropy runs and quick local auditing. It does
not fail the build; it prints a Markdown report to stdout.
#>

param(
  [int]$Limit = 400,
  [Alias("allowlist")]
  [string]$AllowlistPath = "scripts/internal/check/allowlists/file_size_budget_allowlist.txt",
  [Alias("h")]
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Usage {
  Write-Host "Usage: scripts/internal/check/report_file_size_budget_allowlist.ps1 [-Limit <n>] [-AllowlistPath <path>]"
  Write-Host ""
  Write-Host "Prints a Markdown report of allowlisted Rust files and their line counts,"
  Write-Host "highlighting which are still above the limit and which can be removed from"
  Write-Host "the allowlist."
}

if ($Help) {
  Write-Usage
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path

Push-Location $rootDir
try {
  if (-not (Test-Path -LiteralPath $AllowlistPath -PathType Leaf)) {
    Write-Host "# File size budget allowlist report"
    Write-Host ""
    Write-Host ('Allowlist file not found: `{0}`' -f $AllowlistPath)
    exit 0
  }

  $entries = New-Object System.Collections.Generic.List[object]
  foreach ($line in Get-Content -LiteralPath $AllowlistPath) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    if ($line.TrimStart().StartsWith("#")) { continue }

    $file = $line.Trim()
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
      $entries.Add([pscustomobject]@{
          Status = "missing"
          Lines = 0
          File = $file
        })
      continue
    }

    $lineCount = ([System.IO.File]::ReadAllLines((Resolve-Path -LiteralPath $file))).Count
    $status = if ($lineCount -gt $Limit) { "over" } else { "ok" }
    $entries.Add([pscustomobject]@{
        Status = $status
        Lines = $lineCount
        File = $file
      })
  }

  $total = $entries.Count
  $over = @($entries | Where-Object { $_.Status -eq "over" })
  $ok = @($entries | Where-Object { $_.Status -eq "ok" })
  $missing = @($entries | Where-Object { $_.Status -eq "missing" })
  $timestampUtc = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")

  Write-Host "# File size budget allowlist report"
  Write-Host ""
  Write-Host ('- Timestamp (UTC): `{0}`' -f $timestampUtc)
  Write-Host ('- Limit: `{0}` lines' -f $Limit)
  Write-Host ('- Allowlist: `{0}`' -f $AllowlistPath)
  Write-Host ("- Entries: total={0} over={1} ok={2} missing={3}" -f $total, $over.Count, $ok.Count, $missing.Count)
  Write-Host ""

  if ($missing.Count -gt 0) {
    Write-Host "## Missing files (stale allowlist entries)"
    Write-Host ""
    foreach ($entry in ($missing | Sort-Object File)) {
      Write-Host ('- `{0}`' -f $entry.File)
    }
    Write-Host ""
  }

  if ($over.Count -gt 0) {
    Write-Host "## Still over budget (prioritized)"
    Write-Host ""
    Write-Host "| Lines | File |"
    Write-Host "| ---: | --- |"
    foreach ($entry in ($over | Sort-Object Lines, File -Descending)) {
      Write-Host ('| {0} | `{1}` |' -f $entry.Lines, $entry.File)
    }
    Write-Host ""
  } else {
    Write-Host "## Still over budget"
    Write-Host ""
    Write-Host "None."
    Write-Host ""
  }

  if ($ok.Count -gt 0) {
    Write-Host "## Now within budget (can remove from allowlist)"
    Write-Host ""
    Write-Host "| Lines | File |"
    Write-Host "| ---: | --- |"
    foreach ($entry in ($ok | Sort-Object Lines, File -Descending)) {
      Write-Host ('| {0} | `{1}` |' -f $entry.Lines, $entry.File)
    }
    Write-Host ""
  }
} finally {
  Pop-Location
}
