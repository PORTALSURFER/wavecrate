<#
.SYNOPSIS
Builds the Wavecrate mdBook documentation.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$mdbook = Get-Command mdbook -ErrorAction SilentlyContinue
if ($null -eq $mdbook) {
  throw "[mdbook] mdbook is required. Install with: cargo install mdbook --locked"
}

& $mdbook.Path build
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

Write-Host "[mdbook] OK"
