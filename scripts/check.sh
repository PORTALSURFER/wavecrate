#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/check.sh <subcommand> [args...]
EOF
}

if (( $# == 0 )); then
  usage
  exit 0
fi

command="$1"
shift

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/internal/check"

case "$command" in
  app-core-boundary) exec "$script_dir/check_app_core_dependency_boundary.sh" "$@" ;;
  codeowners) exec "$script_dir/check_codeowners_coverage.sh" "$@" ;;
  cleanup-hotspots) exec "$script_dir/audit_cleanup_hotspots.sh" "$@" ;;
  dead-deps) exec "$script_dir/check_rust_dead_deps_advisory.sh" "$@" ;;
  docs-index) exec "$script_dir/check_docs_index.sh" "$@" ;;
  file-size-budget) exec "$script_dir/check_file_size_budget.sh" "$@" ;;
  fix-doc-links) exec "$script_dir/fix_trivial_doc_links.sh" "$@" ;;
  golden-tests) exec "$script_dir/ci_golden_tests.sh" "$@" ;;
  integration-branch) exec "$script_dir/check_main_branch.sh" "$@" ;;
  knowledge) exec "$script_dir/knowledge_lint.sh" "$@" ;;
  legacy-app-coupling) exec "$script_dir/check_legacy_app_coupling.sh" "$@" ;;
  manual-docs-scope) exec "$script_dir/check_manual_docs_scope.sh" "$@" ;;
  markdown-links) exec "$script_dir/check_markdown_links.sh" "$@" ;;
  main-branch) exec "$script_dir/check_main_branch.sh" "$@" ;;
  migration-boundary) exec "$script_dir/check_migration_boundary.sh" "$@" ;;
  native-app-boundary) exec "$script_dir/check_native_app_boundary.sh" "$@" ;;
  non-blocking-architecture) exec "$script_dir/check_non_blocking_architecture.sh" "$@" ;;
  private-docs) exec "$script_dir/check_rust_private_docs.sh" "$@" ;;
  prune-file-budget) exec "$script_dir/prune_file_size_budget_allowlist.sh" "$@" ;;
  public-docs) exec "$script_dir/check_rust_public_docs.sh" "$@" ;;
  report-env-vars) exec "$script_dir/report_env_vars_drift.sh" "$@" ;;
  report-file-budget) exec "$script_dir/report_file_size_budget_allowlist.sh" "$@" ;;
  report-markdown-links) exec "$script_dir/report_markdown_links_all.sh" "$@" ;;
  report-size-hotspots) exec "$script_dir/report_size_hotspots.sh" "$@" ;;
  rust-no-todos) exec "$script_dir/check_rust_no_todos.sh" "$@" ;;
  script-guardrails) exec "$script_dir/check_script_guardrails.sh" "$@" ;;
  taste) exec "$script_dir/check_rust_taste_invariants.sh" "$@" ;;
  workflow-toolchain) exec "$script_dir/check_workflow_toolchain_pinning.sh" "$@" ;;
  wavecrate-facades) exec "$script_dir/check_wavecrate_facades.sh" "$@" ;;
  -h|--help) usage ;;
  *)
    echo "Unknown check command: $command" >&2
    usage >&2
    exit 2
    ;;
esac
