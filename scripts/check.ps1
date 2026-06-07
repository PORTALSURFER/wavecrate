<#
.SYNOPSIS
Dispatches specialist guardrail and reporting scripts.

.DESCRIPTION
The individual checks live under `scripts/internal/check/`; this wrapper
provides a single predictable entrypoint for the common subcommands.
#>

param(
  [Parameter(Position = 0)]
  [string]$Command,
  [switch]$Help,
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$Arguments
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$psExe = (Get-Process -Id $PID).Path

$commands = @{
  "app-core-boundary" = "check_app_core_dependency_boundary.ps1"
  "codeowners" = "check_codeowners_coverage.ps1"
  "cleanup-hotspots" = "audit_cleanup_hotspots.ps1"
  "dead-deps" = "check_rust_dead_deps_advisory.sh"
  "docs-index" = "check_docs_index.ps1"
  "file-size-budget" = "check_file_size_budget.ps1"
  "fix-doc-links" = "fix_trivial_doc_links.ps1"
  "golden-tests" = "ci_golden_tests.ps1"
  "integration-branch" = "check_next_branch.ps1"
  "knowledge" = "knowledge_lint.ps1"
  "legacy-app-coupling" = "check_legacy_app_coupling.ps1"
  "manual-docs-scope" = "check_manual_docs_scope.ps1"
  "markdown-links" = "check_markdown_links.ps1"
  "main-branch" = "check_next_branch.ps1"
  "migration-boundary" = "check_migration_boundary.ps1"
  "native-app-boundary" = "check_native_app_boundary.ps1"
  "private-docs" = "check_rust_private_docs.ps1"
  "prune-file-budget" = "prune_file_size_budget_allowlist.ps1"
  "public-docs" = "check_rust_public_docs.ps1"
  "report-env-vars" = "report_env_vars_drift.sh"
  "report-file-budget" = "report_file_size_budget_allowlist.ps1"
  "report-markdown-links" = "report_markdown_links_all.sh"
  "rust-no-todos" = "check_rust_no_todos.ps1"
  "script-guardrails" = "check_script_guardrails.ps1"
  "taste" = "check_rust_taste_invariants.ps1"
  "workflow-toolchain" = "check_workflow_toolchain_pinning.ps1"
}

if ([string]::IsNullOrWhiteSpace($Command)) {
  Write-Host "Usage: scripts/check.ps1 <subcommand> [args...]"
  exit 0
}

if ($Help) {
  $Arguments = @("-Help") + $Arguments
}

$scriptName = $commands[$Command]
if ($null -eq $scriptName) {
  throw "Unknown check command: $Command"
}

if ($scriptName.EndsWith(".sh")) {
  $bash = Get-Command bash -ErrorAction SilentlyContinue
  if ($null -eq $bash) {
    throw "bash is required for check command '$Command'."
  }
  & $bash.Path (Join-Path $PSScriptRoot "internal/check/$scriptName") @Arguments
  exit $LASTEXITCODE
} else {
  & $psExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "internal/check/$scriptName") @Arguments
  exit $LASTEXITCODE
}
