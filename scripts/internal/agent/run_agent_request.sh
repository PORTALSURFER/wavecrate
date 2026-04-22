#!/usr/bin/env bash

# Run the agent request contract:
# 1) run mandatory preflight,
# 2) run the fast local development checks by default, or the full local CI
#    gate when requested.
#
# This script is intentionally small and deterministic so it can be used as the
# first step of each agent request/session.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: scripts/agent.sh request [--skip-ci] [--quick-ci] [--full-ci]

Run the mandatory agent preflight and optional local development checks.

Options:
  --skip-ci                 Skip ./scripts/ci.sh smoke, ./scripts/ci.sh quick, and ./scripts/ci.sh local.
  --quick-ci                Run fast filtered tests via ./scripts/ci.sh quick.
  --full-ci                 Run full ./scripts/ci.sh local --skip-agent-preflight.
  -h, --help                Show this help text.
USAGE
}

SKIP_CI=0
QUICK_CI=0
FULL_CI=0

while (( $# > 0 )); do
  case "$1" in
    --skip-ci)
      SKIP_CI=1
      shift
      ;;
    --quick-ci)
      QUICK_CI=1
      shift
      ;;
    --full-ci)
      FULL_CI=1
      shift
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

if (( QUICK_CI == 1 && FULL_CI == 1 )); then
  echo "[agent_request] --quick-ci and --full-ci are mutually exclusive." >&2
  usage >&2
  exit 2
fi

./scripts/internal/agent/run_agent_preflight.sh

if (( SKIP_CI == 0 )); then
  if (( FULL_CI == 1 )); then
    ./scripts/ci.sh local --skip-agent-preflight
  elif (( QUICK_CI == 1 )); then
    ./scripts/ci.sh quick
  else
    ./scripts/ci.sh smoke
  fi
fi
