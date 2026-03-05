
<#
.SYNOPSIS
Enforces that `manual/` only contains user-facing docs and site assets.

.DESCRIPTION
Fails when added/modified files under `manual/` are outside the allowlist:
  manual/index.md
  manual/usage.md
  manual/design_principles.md
  manual/_config.yml
  manual/_layouts/**
  manual/assets/**
  manual/README.md
  manual/<redirect-stubs>.md (developer docs moved to `docs/`)

The script is diff-aware: it checks base/head (when provided), plus staged and
unstaged changes. Deletions are allowed.
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
  function Test-GitCommit([string]$Ref) {
    if ([string]::IsNullOrWhiteSpace($Ref)) { return $false }
    try {
      git rev-parse --verify --quiet "$Ref^{commit}" | Out-Null
      return $true
    } catch {
      return $false
    }
  }

  $paths = New-Object "System.Collections.Generic.HashSet[string]"

  function Add-Paths([string[]]$Lines) {
    foreach ($line in $Lines) {
      $p = $line.Trim()
      if ([string]::IsNullOrWhiteSpace($p)) { continue }
      [void]$paths.Add($p)
    }
  }

  if (-not [string]::IsNullOrWhiteSpace($Base) -and (Test-GitCommit $Base) -and (Test-GitCommit $Head)) {
    Add-Paths (git diff --name-only --diff-filter=AM "$Base...$Head" -- manual)
  } elseif (Test-GitCommit $Head) {
    Add-Paths (git show --name-only --pretty=format: $Head -- manual)
  }

  Add-Paths (git diff --name-only --diff-filter=AM --cached -- manual)
  Add-Paths (git diff --name-only --diff-filter=AM -- manual)

  if ($paths.Count -eq 0) {
    Write-Host "[manual_scope] No added/modified files detected under manual/."
    exit 0
  }

  function Is-Allowlisted([string]$Path) {
    switch ($Path) {
      "manual/index.md" { return $true }
      "manual/usage.md" { return $true }
      "manual/design_principles.md" { return $true }
      "manual/_config.yml" { return $true }
      "manual/README.md" { return $true }
      "manual/ann_index_container.md" { return $true }
      "manual/drag_audit.md" { return $true }
      "manual/feature_vector.md" { return $true }
      "manual/gui_migration_parity.md" { return $true }
      "manual/hints.md" { return $true }
      "manual/icon_assets.md" { return $true }
      "manual/native_shell_legacy_baseline.md" { return $true }
      "manual/performance_qa.md" { return $true }
      "manual/plan.md" { return $true }
      "manual/styleguide.md" { return $true }
      "manual/todo.md" { return $true }
      "manual/transient_audit.md" { return $true }
      "manual/transient_plan.md" { return $true }
      "manual/updater-contract.md" { return $true }
      default {
        if ($Path -like "manual/_layouts/*") { return $true }
        if ($Path -like "manual/assets/*") { return $true }
        return $false
      }
    }
  }

  $violations = @()
  foreach ($p in $paths) {
    if (-not (Is-Allowlisted $p)) {
      $violations += $p
    }
  }

  if ($violations.Count -gt 0) {
    Write-Error "[manual_scope] Disallowed added/modified file(s) under manual/:"
    Write-Host "[manual_scope] manual/ is user-facing only; developer docs belong in docs/."
    Write-Host "[manual_scope] Allowlisted paths:"
    Write-Host " - manual/index.md"
    Write-Host " - manual/usage.md"
    Write-Host " - manual/_config.yml"
    Write-Host " - manual/_layouts/**"
    Write-Host " - manual/assets/**"
    Write-Host " - manual/README.md"
    foreach ($v in ($violations | Sort-Object)) {
      Write-Host (" - {0}" -f $v)
    }
    exit 1
  }

  Write-Host "[manual_scope] OK"
  exit 0
} finally {
  Pop-Location
}
