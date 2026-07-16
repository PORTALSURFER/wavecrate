#!/usr/bin/env bash

# Run the bounded repository-state checks that are safe for Git hooks.
#
# Full agent preflight remains explicitly owned by `scripts/agent.sh preflight`
# and `scripts/agent.sh request`. Hooks must not start Cargo-backed checks while
# Git is transitioning between a reviewed candidate and canonical main.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: scripts/internal/agent/run_agent_hook_checks.sh --event <post-merge|post-checkout>

Run cheap repository-state checks for a Git hook. This command intentionally
does not run the full Cargo-backed agent preflight.
USAGE
}

EVENT=""
while (( $# > 0 )); do
  case "$1" in
    --event)
      if (( $# < 2 )); then
        echo "[agent_hook_checks] --event requires a value." >&2
        usage >&2
        exit 2
      fi
      EVENT="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[agent_hook_checks] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$EVENT" in
  post-merge|post-checkout)
    ;;
  *)
    echo "[agent_hook_checks] --event must be post-merge or post-checkout." >&2
    exit 2
    ;;
esac

CHECK_COMMAND="${WAVECRATE_AGENT_HOOK_CHECK_COMMAND:-$ROOT_DIR/scripts/internal/check/check_main_branch.sh}"
if [[ ! -x "$CHECK_COMMAND" ]]; then
  echo "[agent_hook_checks] ERROR: missing executable state check: $CHECK_COMMAND" >&2
  exit 1
fi

echo "[agent_hook_checks] $EVENT: running cheap repository-state checks; full agent preflight remains explicit."
"$CHECK_COMMAND"
echo "[agent_hook_checks] OK"
