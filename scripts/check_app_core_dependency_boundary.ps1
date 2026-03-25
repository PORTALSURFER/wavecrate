<#
.SYNOPSIS
Prevents introducing new dependencies from `src/app_core` into legacy/UI runtime layers.

.DESCRIPTION
Diff-aware check: inspects only added lines in diffs for forbidden dependencies.
`crate::legacy_runtime::` and `crate::gui_app::` remain historical compatibility
tokens; `crate::gui_runtime::` is the current live runtime layer.

Allowlist file (last resort):
  docs/app_core_dependency_boundary_allowlist.txt
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
  $allowlistPath = Join-Path $rootDir "docs/app_core_dependency_boundary_allowlist.txt"
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
    git rev-parse --verify --quiet "$Ref^{commit}" | Out-Null
    return ($LASTEXITCODE -eq 0)
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
      if (-not ($current -like "src/app_core/*.rs" -or $current -like "src/app_core/*/*.rs" -or $current -like "src/app_core/*/*/*.rs" -or $current -like "src/app_core/*/*/*/*.rs" -or $current -like "src/app_core/*/*/*/*/*.rs")) { continue }
      if ($allowlist.Contains($current)) { continue }
      if ($line -match '^\+\s*//') { continue }

      $text = $line.Substring(1)
      if ($text -match '\bcrate::legacy_runtime::') {
        $violations += ("{0}: legacy_runtime: {1}" -f $current, $text.Trim())
      }
      if ($text -match '\bcrate::gui_app::') {
        $violations += ("{0}: gui_app: {1}" -f $current, $text.Trim())
      }
      if ($text -match '\bcrate::gui_runtime::') {
        $violations += ("{0}: gui_runtime: {1}" -f $current, $text.Trim())
      }
    }

    if ($violations.Count -gt 0) {
      Write-Error ("[app_core_boundary] Violations detected ({0}):" -f $Label)
      Write-Host "[app_core_boundary] app_core must not take new dependencies on legacy/UI runtime layers."
      Write-Host "[app_core_boundary] Move code into the current runtime or adapter layer (usually src/gui_runtime or the legacy src/app boundary), or invert the dependency."
      Write-Host ("[app_core_boundary] Allowlist (last resort): {0}" -f $allowlistPath)
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
      $lines = git diff --unified=0 --diff-filter=AMR @Args -- src/app_core
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
  Write-Host "[app_core_boundary] OK"
  exit 0
} finally {
  Pop-Location
}

