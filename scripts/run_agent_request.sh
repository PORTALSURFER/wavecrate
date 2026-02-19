#!/usr/bin/env bash

# Run the full agent request contract:
# 1) run mandatory preflight (checks + MEMORY.md handoff refresh),
# 2) run the full local CI gate (skipping redundant preflight in ci_local).
#
# This script is intentionally small and deterministic so it can be used as the
# first step of each agent request/session.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: scripts/run_agent_request.sh [--skip-ci] [--updater <name>] [--memory-max-age-hours <hours>]

Run the mandatory agent preflight and optional full local CI gate.

Options:
  --skip-ci                 Skip full ./scripts/ci_local.sh --skip-agent-preflight.
  --updater <name>          Name to write into MEMORY.md (default: Codex).
  --memory-max-age-hours N  Freshness threshold for MEMORY.md in hours (default: 1).
  -h, --help                Show this help text.
USAGE
}

SKIP_CI=0
UPDATER="Codex"
MEMORY_MAX_AGE_HOURS=1

while (( $# > 0 )); do
  case "$1" in
    --skip-ci)
      SKIP_CI=1
      shift
      ;;
    --updater)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[agent_request] --updater requires a value." >&2
        usage >&2
        exit 2
      fi
      UPDATER="${2:-}"
      shift 2
      ;;
    --memory-max-age-hours)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[agent_request] --memory-max-age-hours requires a value." >&2
        usage >&2
        exit 2
      fi
      if ! [[ "${2:-}" =~ ^[0-9]+$ ]]; then
        echo "[agent_request] --memory-max-age-hours must be a non-negative integer." >&2
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
      echo "[agent_request] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

./scripts/run_agent_preflight.sh \
  --refresh-memory \
  --updater "$UPDATER" \
  --memory-max-age-hours "$MEMORY_MAX_AGE_HOURS"

if (( SKIP_CI == 0 )); then
  ./scripts/ci_local.sh --skip-agent-preflight
fi
