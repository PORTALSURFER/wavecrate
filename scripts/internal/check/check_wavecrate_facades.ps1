<#
.SYNOPSIS
Enforces Wavecrate facade-size and export-ownership guardrails.

.DESCRIPTION
Protects native-app and app-core facade modules from quiet root-surface growth.
Each facade budget is tied to the cleanup issue that owns shrinking or
reclassifying that surface. Increase a limit only with the matching boundary
cleanup work, not as a convenience for unrelated exports.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($Help) {
  Write-Host "Usage: scripts/internal/check/check_wavecrate_facades.ps1"
  Write-Host ""
  Write-Host "Fails when selected Wavecrate/app-core facades grow or when test/legacy crossings bypass their owners."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path

function Convert-ToRepoPath {
  param([string]$Path)

  $fullPath = (Resolve-Path -LiteralPath $Path).Path
  $prefix = $rootDir.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
  if (-not $fullPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "[wavecrate_facades] Path is outside repository root: $Path"
  }
  return $fullPath.Substring($prefix.Length).Replace("\", "/")
}

function Test-TestPath {
  param([string]$RepoPath)

  if ($RepoPath.EndsWith("_tests.rs")) { return $true }
  if ($RepoPath.EndsWith("/tests.rs")) { return $true }
  if ($RepoPath.Contains("/tests/")) { return $true }
  return $false
}

function Get-LineCount {
  param([string]$RepoPath)

  return ([System.IO.File]::ReadAllLines((Join-Path $rootDir $RepoPath))).Count
}

function Get-MatchingLineCount {
  param(
    [string]$RepoPath,
    [string]$Pattern
  )

  $count = 0
  foreach ($line in Get-Content -LiteralPath (Join-Path $rootDir $RepoPath)) {
    if ($line -match $Pattern) { $count++ }
  }
  return $count
}

$facades = @(
  @{
    Path = "src/native_app/test_support.rs"
    MaxLines = 80
    MaxExports = 9
    MaxPublicModules = 0
    CountRestrictedPublicModules = $false
    Owner = "OPT-541"
    Reason = "native test fixtures must stay split by focused support module"
  },
  @{
    Path = "src/native_app/sample_library/folder_browser.rs"
    MaxLines = 180
    MaxExports = 13
    MaxPublicModules = 6
    Owner = "OPT-529"
    Reason = "folder browser root remains a facade over owned browsing modules"
  },
  @{
    Path = "src/app_core/app_api.rs"
    MaxLines = 180
    MaxExports = 24
    MaxPublicModules = 4
    Owner = "OPT-537/OPT-538"
    Reason = "legacy crossings are an audited migration surface"
  },
  @{
    Path = "src/app_core/actions/mod.rs"
    MaxLines = 260
    MaxExports = 66
    MaxPublicModules = 1
    Owner = "OPT-539"
    Reason = "action catalog/type exports must shrink by domain instead of growing at the root"
  }
)

$exportPattern = '^\s*pub(?:\s*\([^)]*\))?\s+(?:use|type)\b'
$restrictedOrPublicModulePattern = '^\s*pub(?:\s*\([^)]*\))?\s+mod\s+\w+\s*(?:;|\{)'
$publicModulePattern = '^\s*pub\s+mod\s+\w+\s*(?:;|\{)'
$violations = New-Object System.Collections.Generic.List[string]

Push-Location $rootDir
try {
  foreach ($facade in $facades) {
    $path = [string]$facade.Path
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
      $violations.Add(("{0}: guarded facade is missing ({1})" -f $path, $facade.Owner))
      continue
    }

    $lineCount = Get-LineCount -RepoPath $path
    $exportCount = Get-MatchingLineCount -RepoPath $path -Pattern $exportPattern
    $modulePattern = $restrictedOrPublicModulePattern
    if (
      $facade.ContainsKey("CountRestrictedPublicModules") -and
      -not [bool]$facade.CountRestrictedPublicModules
    ) {
      $modulePattern = $publicModulePattern
    }
    $moduleCount = Get-MatchingLineCount -RepoPath $path -Pattern $modulePattern

    if ($lineCount -gt [int]$facade.MaxLines) {
      $violations.Add(("{0}: {1} lines exceeds facade budget {2} ({3}; {4})" -f $path, $lineCount, $facade.MaxLines, $facade.Owner, $facade.Reason))
    }
    if ($exportCount -gt [int]$facade.MaxExports) {
      $violations.Add(("{0}: {1} root exports exceeds budget {2} ({3}; {4})" -f $path, $exportCount, $facade.MaxExports, $facade.Owner, $facade.Reason))
    }
    if ($moduleCount -gt [int]$facade.MaxPublicModules) {
      $violations.Add(("{0}: {1} public modules exceeds budget {2} ({3}; {4})" -f $path, $moduleCount, $facade.MaxPublicModules, $facade.Owner, $facade.Reason))
    }
  }

  foreach ($file in Get-ChildItem -LiteralPath (Join-Path $rootDir "src/native_app") -Recurse -File -Filter "*.rs") {
    $repoPath = Convert-ToRepoPath -Path $file.FullName
    if (Test-TestPath -RepoPath $repoPath) { continue }
    if ($repoPath -eq "src/native_app/test_support.rs") { continue }
    if ($repoPath.StartsWith("src/native_app/test_support/")) { continue }

    $lineNumber = 0
    foreach ($line in Get-Content -LiteralPath $file.FullName) {
      $lineNumber++
      if ($line -match '^\s*//') { continue }
      if ($line -match '\b(?:crate::native_app::test_support|super::test_support)\b') {
        $violations.Add(("{0}:{1}: production native-app code must not import test_support: {2}" -f $repoPath, $lineNumber, $line.Trim()))
      }
    }
  }

  foreach ($file in Get-ChildItem -LiteralPath (Join-Path $rootDir "src/app_core") -Recurse -File -Filter "*.rs") {
    $repoPath = Convert-ToRepoPath -Path $file.FullName
    if (Test-TestPath -RepoPath $repoPath) { continue }
    if ($repoPath -eq "src/app_core/app_api.rs") { continue }

    $lineNumber = 0
    foreach ($line in Get-Content -LiteralPath $file.FullName) {
      $lineNumber++
      if ($line -match '^\s*//') { continue }
      if ($line -match '\bcrate::app::') {
        $violations.Add(("{0}:{1}: app-core legacy crossings must go through app_core::app_api: {2}" -f $repoPath, $lineNumber, $line.Trim()))
      }
    }
  }

  if ($violations.Count -gt 0) {
    Write-Host "[wavecrate_facades] Facade guardrail violations detected:"
    Write-Host "[wavecrate_facades] Keep root facades small, route legacy app crossings through app_core::app_api, and keep test_support out of production imports."
    foreach ($violation in ($violations | Sort-Object)) {
      Write-Host (" - {0}" -f $violation)
    }
    exit 1
  }

  Write-Host "[wavecrate_facades] OK"
  exit 0
} finally {
  Pop-Location
}
