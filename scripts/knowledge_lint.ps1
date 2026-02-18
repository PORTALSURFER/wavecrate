Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

param(
  [string]$Base,
  [string]$Head = "HEAD"
)

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  & (Join-Path $rootDir "scripts/check_docs_index.ps1")
  if ([string]::IsNullOrWhiteSpace($Base)) {
    & (Join-Path $rootDir "scripts/check_markdown_links.ps1") -Head $Head
  } else {
    & (Join-Path $rootDir "scripts/check_markdown_links.ps1") -Base $Base -Head $Head
  }
} finally {
  Pop-Location
}

