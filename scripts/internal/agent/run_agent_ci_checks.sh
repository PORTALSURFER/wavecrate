#!/usr/bin/env bash

# Required preflight for agent-driven work.
#
# The script enforces lightweight, high-value checks that should run on every
# agent request before substantive edits and before handoff.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: scripts/internal/agent/run_agent_ci_checks.sh

Run agent-request readiness checks required by local CI conventions.

Options:
  --help                 Show this help text.
USAGE
}

while (( $# > 0 )); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[agent_ci] Unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

run_check() {
  local label="$1"
  shift
  echo "[agent_ci] $label"
  "$@"
}

run_check "development branch policy" ./scripts/internal/check/check_main_branch.sh
run_check "migration boundary guardrails" ./scripts/internal/check/check_migration_boundary.sh
run_check "script guardrails" ./scripts/internal/check/check_script_guardrails.sh
run_check "workflow toolchain pinning" ./scripts/internal/check/check_workflow_toolchain_pinning.sh
run_check "manual docs scope guard" ./scripts/internal/check/check_manual_docs_scope.sh
run_check "legacy app coupling guardrail" ./scripts/internal/check/check_legacy_app_coupling.sh
run_check "native app boundary guardrail" ./scripts/internal/check/check_native_app_boundary.sh
run_check "readiness executor boundary guardrail" ./scripts/internal/check/check_readiness_executor_boundary.sh
run_check "source database open-role guardrail" ./scripts/internal/check/check_source_db_open_roles.sh
run_check "non-blocking architecture guardrails" ./scripts/internal/check/check_non_blocking_architecture.sh
run_check "Wavecrate facade guardrail" ./scripts/internal/check/check_wavecrate_facades.sh
run_check "rust todo/todo guardrail (non-test only)" ./scripts/internal/check/check_rust_no_todos.sh
run_check "rust dead dependency/unused code sweep (advisory)" \
  ./scripts/internal/check/check_rust_dead_deps_advisory.sh --advisory
run_check "rust public docs guardrail" ./scripts/internal/check/check_rust_public_docs.sh
run_check "rust private docs guardrail" ./scripts/internal/check/check_rust_private_docs.sh
run_check "app_core dependency boundary" ./scripts/internal/check/check_app_core_dependency_boundary.sh
run_check "knowledge lint" ./scripts/internal/check/knowledge_lint.sh

echo "[agent_ci] OK"
