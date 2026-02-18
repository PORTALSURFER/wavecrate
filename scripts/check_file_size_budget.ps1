Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Enforces a per-file line budget for Rust sources.

.DESCRIPTION
Checks Rust files under `src/`, `tests/`, and `vendor/radiant/src` and fails if
any non-allowlisted file exceeds the line budget.

By default, checks files added/modified in the supplied git diff range (if any),
plus staged/unstaged working tree edits. Known legacy exceptions live in
`docs/file_size_budget_allowlist.txt`.
#>

param(
  [string]$Base,
  [string]$Head = "HEAD",
  [int]$Limit = 400,
  [switch]$All
)

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  $allowlistPath = Join-Path $rootDir "docs/file_size_budget_allowlist.txt"
  $allowlist = New-Object "System.Collections.Generic.HashSet[string]"
  if (Test-Path -LiteralPath $allowlistPath) {
    foreach ($line in Get-Content -LiteralPath $allowlistPath) {
      if ([string]::IsNullOrWhiteSpace($line)) { continue }
      if ($line.TrimStart().StartsWith("#")) { continue }
      [void]$allowlist.Add($line.Trim())
    }
  }

  $scopePaths = @("src", "tests", "vendor/radiant/src")
  $files = New-Object "System.Collections.Generic.HashSet[string]"

  function Test-GitCommit([string]$Ref) {
    if ([string]::IsNullOrWhiteSpace($Ref)) { return $false }
    try {
      git rev-parse --verify --quiet "$Ref^{commit}" | Out-Null
      return $true
    } catch {
      return $false
    }
  }

  function Add-GitFileList([string[]]$Lines) {
    foreach ($line in $Lines) {
      $path = $line.Trim()
      if ([string]::IsNullOrWhiteSpace($path)) { continue }
      if (-not $path.EndsWith(".rs")) { continue }
      [void]$files.Add($path)
    }
  }

  if ($All) {
    Add-GitFileList (git ls-files -- $scopePaths)
  } else {
    if ([string]::IsNullOrWhiteSpace($Base)) {
      if (Test-GitCommit "origin/main") { $Base = "origin/main" }
      elseif (Test-GitCommit "main") { $Base = "main" }
    }

    if (-not [string]::IsNullOrWhiteSpace($Base) -and (Test-GitCommit $Base) -and (Test-GitCommit $Head)) {
      Add-GitFileList (git diff --name-only --diff-filter=AM "$Base...$Head" -- $scopePaths)
    } elseif (Test-GitCommit $Head) {
      Add-GitFileList (git show --name-only --pretty=format: $Head -- $scopePaths)
    }

    Add-GitFileList (git diff --name-only --diff-filter=AM --cached -- $scopePaths)
    Add-GitFileList (git diff --name-only --diff-filter=AM -- $scopePaths)
  }

  if ($files.Count -eq 0) {
    Write-Host "[file_budget] No changed Rust files detected."
    exit 0
  }

  $violations = @()
  $checked = 0
  foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { continue }
    $checked++

    if ($allowlist.Contains($file)) { continue }

    $lineCount = (Get-Content -LiteralPath $file | Measure-Object -Line).Lines
    if ($lineCount -gt $Limit) {
      $violations += ("{0}: {1}" -f $file, $lineCount)
    }
  }

  if ($checked -eq 0) {
    Write-Host "[file_budget] No matching Rust files found to check."
    exit 0
  }

  if ($violations.Count -gt 0) {
    Write-Error ("[file_budget] File size budget violations (limit: {0} lines):" -f $Limit)
    foreach ($v in ($violations | Sort-Object)) {
      Write-Host (" - {0}" -f $v)
    }
    Write-Host ("[file_budget] Fix by splitting files into focused modules, or (temporarily) add to allowlist: {0}" -f $allowlistPath)
    exit 1
  }

  Write-Host ("[file_budget] OK ({0} files checked)" -f $checked)
  exit 0
} finally {
  Pop-Location
}

