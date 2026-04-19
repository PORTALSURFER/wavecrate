<#
.SYNOPSIS
Dispatches specialist guardrail and reporting scripts.

.DESCRIPTION
The individual checks live under `scripts/check/`; this wrapper provides a
single predictable entrypoint for the most common subcommands.
#>

param(
  [Parameter(Position = 0)]
  [string]$Command,
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$Arguments
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$commands = @{
  "next-branch" = "check_next_branch.ps1"
  "migration-boundary" = "check_migration_boundary.ps1"
  "file-size-budget" = "check_file_size_budget.ps1"
  "docs-index" = "check_docs_index.ps1"
  "codeowners" = "check_codeowners_coverage.ps1"
  "markdown-links" = "check_markdown_links.ps1"
  "knowledge" = "knowledge_lint.ps1"
}

if ([string]::IsNullOrWhiteSpace($Command) -or $Command -in @("-h", "--help", "-Help")) {
  Write-Host "Usage: scripts/check.ps1 <next-branch|migration-boundary|file-size-budget|docs-index|codeowners|markdown-links|knowledge> [args...]"
  exit 0
}

$scriptName = $commands[$Command]
if ($null -eq $scriptName) {
  throw "Unknown check command: $Command"
}

& (Join-Path $PSScriptRoot "check/$scriptName") @Arguments
