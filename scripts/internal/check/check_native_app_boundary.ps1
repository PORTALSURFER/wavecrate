<#
.SYNOPSIS
Enforces the native-app app-chrome versus domain module boundary.

.DESCRIPTION
The durable module contract lives in `docs/TARGET.md`: app chrome owns view
composition, while native-app product/domain/workflow modules must not import
app-chrome rendering helpers directly. This check also rejects generic root
native-app modules whose names hide ownership, such as `browser` or
`context_menu`.
#>

param(
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($Help) {
  Write-Host "Usage: scripts/internal/check/check_native_app_boundary.ps1"
  Write-Host ""
  Write-Host "Fails when native-app domain modules import app_chrome or when ambiguous root native-app module names are introduced."
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path

function Convert-ToRepoPath {
  param([string]$Path)

  $fullPath = (Resolve-Path -LiteralPath $Path).Path
  $prefix = $rootDir.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
  if (-not $fullPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "[native_app_boundary] Path is outside repository root: $Path"
  }
  $relative = $fullPath.Substring($prefix.Length)
  return $relative.Replace("\", "/")
}

function Test-DomainNativeAppPath {
  param([string]$RepoPath)

  foreach ($root in @("audio", "library_browser", "metadata", "waveform", "workflows")) {
    if ($RepoPath -eq "src/native_app/$root.rs") { return $true }
    if ($RepoPath.StartsWith("src/native_app/$root/")) { return $true }
  }
  return $false
}

function Test-TestOrSupportPath {
  param([string]$RepoPath)

  if ($RepoPath -eq "src/native_app/test_support.rs") { return $true }
  if ($RepoPath -eq "src/native_app/tests.rs") { return $true }
  if ($RepoPath.StartsWith("src/native_app/tests/")) { return $true }
  if ($RepoPath.Contains("/tests/")) { return $true }
  if ($RepoPath.EndsWith("_tests.rs")) { return $true }
  return $false
}

Push-Location $rootDir
try {
  $violations = New-Object System.Collections.Generic.List[string]
  $ambiguousModules = @("browser", "context_menu", "widgets")

  foreach ($file in Get-ChildItem -LiteralPath (Join-Path $rootDir "src/native_app") -Recurse -File -Filter "*.rs") {
    $repoPath = Convert-ToRepoPath -Path $file.FullName
    if (-not (Test-DomainNativeAppPath -RepoPath $repoPath)) { continue }
    if (Test-TestOrSupportPath -RepoPath $repoPath) { continue }

    $lineNumber = 0
    foreach ($line in Get-Content -LiteralPath $file.FullName) {
      $lineNumber++
      if ($line -match '^\s*//') { continue }
      if ($line -match '\bcrate::native_app::app_chrome\b') {
        $violations.Add(("{0}:{1}: domain modules must not import app_chrome: {2}" -f $repoPath, $lineNumber, $line.Trim()))
      }
    }
  }

  foreach ($moduleName in $ambiguousModules) {
    $moduleFile = Join-Path $rootDir ("src/native_app/{0}.rs" -f $moduleName)
    if (Test-Path -LiteralPath $moduleFile) {
      $violations.Add(("src/native_app/{0}.rs: ambiguous root native-app module; move feature-specific code under its owner, e.g. app_chrome or library_browser" -f $moduleName))
    }

    $nativeAppRoot = Join-Path $rootDir "src/native_app.rs"
    if (Test-Path -LiteralPath $nativeAppRoot) {
      $lineNumber = 0
      foreach ($line in Get-Content -LiteralPath $nativeAppRoot) {
        $lineNumber++
        if ($line -match '^\s*//') { continue }
        if ($line -match ('^\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+{0}\s*;' -f [regex]::Escape($moduleName))) {
          $violations.Add(("src/native_app.rs:{0}: ambiguous root native-app module declaration mod {1};" -f $lineNumber, $moduleName))
        }
      }
    }
  }

  if ($violations.Count -gt 0) {
    Write-Host "[native_app_boundary] Native app boundary violations detected:"
    Write-Host "[native_app_boundary] app_chrome is the view-composition layer; product/domain modules must depend on messages, view models, or domain APIs instead."
    Write-Host "[native_app_boundary] Root native-app module names must describe durable ownership, not generic widgets. See docs/TARGET.md native app module map."
    foreach ($violation in ($violations | Sort-Object)) {
      Write-Host (" - {0}" -f $violation)
    }
    exit 1
  }

  Write-Host "[native_app_boundary] OK"
  exit 0
} finally {
  Pop-Location
}
