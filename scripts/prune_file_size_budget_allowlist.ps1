param(
  [int]$Limit = 400,
  [string]$AllowlistPath = "docs/file_size_budget_allowlist.txt"
)

$ErrorActionPreference = 'Stop'

function Write-Usage {
  Write-Host "Usage: scripts/prune_file_size_budget_allowlist.ps1 [-Limit <n>] [-AllowlistPath <path>]"
  Write-Host ""
  Write-Host "Rewrites the allowlist file in-place, removing entries whose file is:"
  Write-Host "- missing, or"
  Write-Host "- <= <limit> lines."
  Write-Host ""
  Write-Host "Comment and blank lines are preserved."
}

try {
  $root = Resolve-Path (Join-Path $PSScriptRoot "..")
  Set-Location $root
} catch {
  Write-Error $_
  exit 1
}

if (-not (Test-Path -LiteralPath $AllowlistPath)) {
  Write-Host "[prune_file_size_budget_allowlist] allowlist not found: $AllowlistPath"
  exit 0
}

$lines = Get-Content -LiteralPath $AllowlistPath -Raw -Encoding UTF8
$split = $lines -split "`r?`n"

$out = New-Object System.Collections.Generic.List[string]
$removedOk = 0
$removedMissing = 0
$kept = 0

foreach ($line in $split) {
  if ($line -match '^\s*$' -or $line -match '^\s*#') {
    $out.Add($line)
    continue
  }

  $file = $line.Trim()
  if (-not (Test-Path -LiteralPath $file)) {
    $removedMissing++
    continue
  }

  $count = (Get-Content -LiteralPath $file | Measure-Object -Line).Lines
  if ($count -le $Limit) {
    $removedOk++
    continue
  }

  $kept++
  $out.Add($line)
}

$newText = ($out -join "`n").TrimEnd() + "`n"
$oldText = (Get-Content -LiteralPath $AllowlistPath -Raw -Encoding UTF8)

if ($newText -eq $oldText) {
  Write-Host "[prune_file_size_budget_allowlist] no changes (kept=$kept removed_ok=$removedOk removed_missing=$removedMissing)"
  exit 0
}

Set-Content -LiteralPath $AllowlistPath -Value $newText -Encoding UTF8
Write-Host "[prune_file_size_budget_allowlist] updated $AllowlistPath (kept=$kept removed_ok=$removedOk removed_missing=$removedMissing)"

