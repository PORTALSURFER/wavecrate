
<#
.SYNOPSIS
Prevents introducing new coupling to the legacy `src/app` module from non-legacy codepaths.

.DESCRIPTION
Diff-aware check: inspects only added lines in diffs for `crate::app` usage.

Scope:
- Checks diffs under `src/`
- Skips legacy paths: `src/app/**`, `src/legacy_runtime/**`
- Allows a small transitional allowlist in `docs/legacy_app_coupling_allowlist.txt`
#>

param(
  [string]$Base,
  [string]$Head = "HEAD"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  $allowlistPath = Join-Path $rootDir "docs/legacy_app_coupling_allowlist.txt"
  $allowlist = New-Object "System.Collections.Generic.HashSet[string]"
  if (Test-Path -LiteralPath $allowlistPath) {
    foreach ($line in Get-Content -LiteralPath $allowlistPath) {
      $t = $line.Trim()
      if ([string]::IsNullOrWhiteSpace($t)) { continue }
      if ($t.StartsWith("#")) { continue }
      [void]$allowlist.Add($t)
    }
  }

  function Test-GitCommit([string]$Ref) {
    if ([string]::IsNullOrWhiteSpace($Ref)) { return $false }
    try {
      git rev-parse --verify --quiet "$Ref^{commit}" | Out-Null
      return $true
    } catch {
      return $false
    }
  }

  function Is-LegacyPath([string]$Path) {
    return ($Path -like "src/app/*") -or ($Path -like "src/legacy_runtime/*")
  }

  function Scan-DiffLines([string]$Label, [string[]]$Lines) {
    $current = ""
    $violations = @()
    foreach ($line in $Lines) {
      if ($line.StartsWith("+++ b/")) {
        $current = $line.Substring(6)
        continue
      }
      if (-not $line.StartsWith("+")) { continue }
      if ($line.StartsWith("+++")) { continue }
      if ([string]::IsNullOrWhiteSpace($current)) { continue }
      if (-not ($current -like "src/*")) { continue }
      if (Is-LegacyPath $current) { continue }
      if ($allowlist.Contains($current)) { continue }
      if ($line -match "\bcrate::app\b") {
        $violations += ("{0}: {1}" -f $current, $line.Substring(1).Trim())
      }
    }

    if ($violations.Count -gt 0) {
      Write-Error ("[legacy_app] New legacy coupling detected ({0}):" -f $Label)
      Write-Host "[legacy_app] Do not introduce new crate::app references outside src/app/."
      Write-Host ("[legacy_app] If this is a transitional shim, add the file to {0}." -f $allowlistPath)
      foreach ($v in ($violations | Sort-Object)) {
        Write-Host (" - {0}" -f $v)
      }
      return $false
    }
    return $true
  }

  function Scan-GitDiff([string]$Label, [string[]]$Args) {
    $lines = @()
    try {
      $lines = git diff --unified=0 --diff-filter=AMR @Args -- src
    } catch {
      $lines = @()
    }
    return (Scan-DiffLines -Label $Label -Lines $lines)
  }

  $ok = $true
  if ((-not [string]::IsNullOrWhiteSpace($Base)) -and (Test-GitCommit $Base) -and (Test-GitCommit $Head)) {
    $ok = $ok -and (Scan-GitDiff -Label ("range " + $Base + "..." + $Head) -Args @("$Base...$Head"))
  }

  $ok = $ok -and (Scan-GitDiff -Label "staged" -Args @("--cached"))
  $ok = $ok -and (Scan-GitDiff -Label "unstaged" -Args @())

  if (-not $ok) { exit 1 }
  Write-Host "[legacy_app] OK"
  exit 0
} finally {
  Pop-Location
}

