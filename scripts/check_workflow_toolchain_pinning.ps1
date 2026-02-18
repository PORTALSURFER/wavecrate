Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Guards against workflow/toolchain drift.

.DESCRIPTION
Ensures workflows that install Rust derive the toolchain from `rust-toolchain.toml`
(not a literal "stable"/"beta"/"nightly" toolchain).
#>

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  $workflowsDir = ".github/workflows"
  if (-not (Test-Path -LiteralPath $workflowsDir -PathType Container)) {
    throw "[toolchain_pinning] Missing $workflowsDir"
  }

  $files = Get-ChildItem -LiteralPath $workflowsDir -File -Filter "*.yml"
  if ($files.Count -eq 0) {
    throw "[toolchain_pinning] No workflow yml files found under $workflowsDir"
  }

  $violations = New-Object System.Collections.Generic.List[string]
  foreach ($file in $files) {
    $text = Get-Content -LiteralPath $file.FullName -Raw

    if ($text -match 'dtolnay/rust-toolchain@' -and $text -notmatch 'rust-toolchain\.toml') {
      $violations.Add(("{0}: uses dtolnay/rust-toolchain but does not reference rust-toolchain.toml" -f $file.FullName))
    }

    $lines = $text -split "`r?`n"
    foreach ($line in $lines) {
      if ($line -match '^\s*toolchain:\s*["'']?(stable|beta|nightly)["'']?\s*$') {
        $violations.Add(("{0}: contains literal toolchain: {1}" -f $file.FullName, $line.Trim()))
      }
    }

    if ($text -match 'rustup\s+toolchain\s+install\s+(stable|beta|nightly)\b') {
      $violations.Add(("{0}: installs toolchain by name (use rust-toolchain.toml)" -f $file.FullName))
    }

    if ($text -match 'actions-rs/toolchain') {
      $violations.Add(("{0}: uses actions-rs/toolchain" -f $file.FullName))
    }
  }

  if ($violations.Count -gt 0) {
    Write-Error "[toolchain_pinning] Workflow toolchain pinning violations:"
    foreach ($v in ($violations | Sort-Object)) {
      Write-Host (" - {0}" -f $v)
    }
    exit 1
  }

  Write-Host "[toolchain_pinning] OK"
  exit 0
} finally {
  Pop-Location
}

