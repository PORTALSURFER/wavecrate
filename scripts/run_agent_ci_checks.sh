#!/usr/bin/env bash

# Required preflight for agent-driven work.
#
# The script enforces lightweight, high-value checks that should run on every
# agent request before substantive edits and before handoff.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

REFRESH_MEMORY=0
MEMORY_MAX_AGE_HOURS="${AGENT_CI_MEMORY_MAX_AGE_HOURS:-24}"
REQUIRED_UPDATER="${AGENT_CI_REQUIRED_UPDATER:-}"
UPDATER="Codex"

usage() {
  cat <<'USAGE'
Usage: scripts/run_agent_ci_checks.sh [--refresh-memory] [--updater <name>] [--required-updater <name>] [--memory-max-age-hours <hours>]

Run agent-request readiness checks required by local CI conventions.

Options:
  --refresh-memory       Update MEMORY.md to a fresh UTC timestamp before checks.
  --updater <name>       Name to write into MEMORY.md when refreshing.
  --memory-max-age-hours N
                        Maximum allowed age of MEMORY.md in hours (default: 24,
                        or AGENT_CI_MEMORY_MAX_AGE_HOURS if set).
  --required-updater <name>
                        Require MEMORY.md 'Updated By:' to match this value.
                        Defaults to AGENT_CI_REQUIRED_UPDATER or unset.
  --help                 Show this help text.
USAGE
}

while (( $# > 0 )); do
  case "$1" in
    --refresh-memory)
      REFRESH_MEMORY=1
      shift
      ;;
    --updater)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[agent_ci] --updater requires a value." >&2
        usage >&2
        exit 2
      fi
      UPDATER="${2:-}"
      shift 2
      ;;
    --required-updater)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[agent_ci] --required-updater requires a value." >&2
        usage >&2
        exit 2
      fi
      REQUIRED_UPDATER="${2:-}"
      shift 2
      ;;
    --memory-max-age-hours)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[agent_ci] --memory-max-age-hours requires a value." >&2
        usage >&2
        exit 2
      fi
      if ! [[ "${2:-}" =~ ^[0-9]+$ ]]; then
        echo "[agent_ci] --memory-max-age-hours must be a non-negative integer." >&2
        usage >&2
        exit 2
      fi
      MEMORY_MAX_AGE_HOURS="${2:-}"
      shift 2
      ;;
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

if (( REFRESH_MEMORY == 1 )); then
  ./scripts/refresh_memory_md.sh "$UPDATER"
fi

run_check() {
  local label="$1"
  local max_age="${2:-}"
  local required_updater="${3:-}"
  shift 3

  local details=""
  if [[ -n "$max_age" ]]; then
    details+="max_age=${max_age}h"
  fi
  if [[ -n "$required_updater" ]]; then
    if [[ -n "$details" ]]; then
      details+=" "
    fi
    details+="required_updater=${required_updater}"
  fi
  if [[ -n "$details" ]]; then
    echo "[agent_ci] $label ($details)"
  else
    echo "[agent_ci] $label"
  fi

  if [[ -n "$max_age" ]]; then
    if [[ -n "$required_updater" ]]; then
      MEMORY_MAX_AGE_HOURS="$max_age" MEMORY_REQUIRED_UPDATER="$required_updater" "$@"
    else
      MEMORY_MAX_AGE_HOURS="$max_age" "$@"
    fi
  else
    if [[ -n "$required_updater" ]]; then
      MEMORY_REQUIRED_UPDATER="$required_updater" "$@"
    else
      "$@"
    fi
  fi
}

run_check "memory log must be fresh (agent mode)" \
  "$MEMORY_MAX_AGE_HOURS" \
  "$REQUIRED_UPDATER" \
  ./scripts/check_memory_log.sh
run_check "development branch policy" "" "" ./scripts/check_next_branch.sh
run_check "migration boundary guardrails" "" "" ./scripts/check_migration_boundary.sh
run_check "script guardrails" "" "" ./scripts/check_script_guardrails.sh
run_check "workflow toolchain pinning" "" "" ./scripts/check_workflow_toolchain_pinning.sh
run_check "high-visibility guardrail score alignment" "" "" ./scripts/check_quality_score_drift.sh
run_check "manual docs scope guard" "" "" ./scripts/check_manual_docs_scope.sh
run_check "legacy app coupling guardrail" "" "" ./scripts/check_legacy_app_coupling.sh
run_check "rust todo/todo guardrail (non-test only)" "" "" ./scripts/check_rust_no_todos.sh
run_check "rust dead dependency/unused code sweep (advisory)" "" "" \
  ./scripts/check_rust_dead_deps_advisory.sh --advisory
run_check "rust public docs guardrail" "" "" ./scripts/check_rust_public_docs.sh
run_check "rust private docs guardrail" "" "" ./scripts/check_rust_private_docs.sh
run_check "app_core dependency boundary" "" "" ./scripts/check_app_core_dependency_boundary.sh
run_check "knowledge lint" "" "" ./scripts/knowledge_lint.sh

echo "[agent_ci] OK"
