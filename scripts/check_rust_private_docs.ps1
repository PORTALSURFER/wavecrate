Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Diff-aware guard for doc comments on newly added Rust items.

.DESCRIPTION
PowerShell wrapper around the canonical bash implementation.
#>

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  if (-not (Get-Command bash -ErrorAction SilentlyContinue)) {
    throw "[private_docs] ERROR: bash is required for scripts/check_rust_private_docs.sh"
  }
  & bash (Join-Path $rootDir "scripts/check_rust_private_docs.sh")
} finally {
  Pop-Location
}
