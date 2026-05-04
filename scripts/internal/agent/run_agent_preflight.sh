#!/usr/bin/env bash

# Run mandatory agent-facing preflight checks.
#
# This script is intentionally lightweight: it validates required guardrails
# and exits on the first failure. Use this at the start of each request and
# before operations that depend on a clean, checked repo state.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: scripts/internal/agent/run_agent_preflight.sh

Run the mandatory preflight checks for an agent request.

Options:
  -h, --help             Show this help text.
USAGE
}

while (( $# > 0 )); do
  case "$1" in
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

./scripts/internal/agent/run_agent_ci_checks.sh

echo "[agent_preflight] OK"
