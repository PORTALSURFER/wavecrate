#!/usr/bin/env bash

# Run mandatory agent-facing preflight checks.
#
# This script is intentionally lightweight: it refreshes MEMORY.md (unless
# disabled), validates required guardrails, and exits on the first failure.
# Use this at the start of each request and before operations that depend on a
# clean, checked repo state.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

UPDATER="Codex"
MEMORY_MAX_AGE_HOURS="${AGENT_PREFLIGHT_MEMORY_MAX_AGE_HOURS:-1}"
REFRESH_MEMORY=1

validate_positive_integer() {
  local value="$1"
  local source="$2"

  if ! [[ "$value" =~ ^[0-9]+$ ]]; then
    echo "[agent_preflight] $source must be a non-negative integer." >&2
    exit 2
  fi
}

validate_updater() {
  local value="$1"
  local source="$2"

  if [[ -z "$value" ]]; then
    echo "[agent_preflight] $source must be a non-empty updater name." >&2
    exit 2
  fi
}

UPDATER="${AGENT_PREFLIGHT_UPDATER:-$UPDATER}"
validate_updater "$UPDATER" "AGENT_PREFLIGHT_UPDATER"
validate_positive_integer "$MEMORY_MAX_AGE_HOURS" "AGENT_PREFLIGHT_MEMORY_MAX_AGE_HOURS"

usage() {
  cat <<'USAGE'
Usage: scripts/run_agent_preflight.sh [--refresh-memory | --no-refresh] [--updater <name>] [--memory-max-age-hours <hours>]

Run the mandatory preflight checks for an agent request.

Options:
  --refresh-memory        Update MEMORY.md before validation (default).
  --no-refresh            Do not refresh MEMORY.md; fail if it is stale.
  --updater <name>        Name to write into MEMORY.md when refreshing.
  --memory-max-age-hours N Maximum allowed age of MEMORY.md in hours (default: 1).
  -h, --help             Show this help text.
USAGE
}

while (( $# > 0 )); do
  case "$1" in
    --refresh-memory)
      REFRESH_MEMORY=1
      shift
      ;;
    --no-refresh)
      REFRESH_MEMORY=0
      shift
      ;;
    --updater)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[agent_preflight] --updater requires a value." >&2
        usage >&2
        exit 2
      fi
      UPDATER="${2:-}"
      validate_updater "$UPDATER" "--updater"
      shift 2
      ;;
    --memory-max-age-hours)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[agent_preflight] --memory-max-age-hours requires a value." >&2
        usage >&2
        exit 2
      fi
      MEMORY_MAX_AGE_HOURS="${2:-}"
      validate_positive_integer "$MEMORY_MAX_AGE_HOURS" "--memory-max-age-hours"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[agent_preflight] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if (( REFRESH_MEMORY == 1 )); then
  ./scripts/run_agent_ci_checks.sh \
    --refresh-memory \
    --updater "$UPDATER" \
    --required-updater "$UPDATER" \
    --memory-max-age-hours "$MEMORY_MAX_AGE_HOURS"
else
  ./scripts/run_agent_ci_checks.sh \
    --required-updater "$UPDATER" \
    --memory-max-age-hours "$MEMORY_MAX_AGE_HOURS"
fi

echo "[agent_preflight] OK"
