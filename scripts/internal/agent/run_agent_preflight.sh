#!/usr/bin/env bash

# Run mandatory agent-facing preflight checks.
#
# This script runs the full guardrail set. Concurrent invocations for one Git
# repository coalesce to one owner so request startup cannot duplicate Cargo
# work with another explicit preflight.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: scripts/internal/agent/run_agent_preflight.sh

Run the mandatory full preflight checks for an agent request. Concurrent
invocations for the same Git repository coalesce to one owner.

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

STATE_DIR="${WAVECRATE_AGENT_PREFLIGHT_STATE_DIR:-}"
if [[ -z "$STATE_DIR" ]]; then
  GIT_COMMON_DIR="$(git rev-parse --git-common-dir)"
  if [[ "$GIT_COMMON_DIR" == /* ]]; then
    STATE_DIR="$GIT_COMMON_DIR/agent-preflight-state"
  else
    STATE_DIR="$ROOT_DIR/$GIT_COMMON_DIR/agent-preflight-state"
  fi
fi
mkdir -p "$STATE_DIR"

LOCK_DIR="$STATE_DIR/run.lock"
OWNER=0
RESULT_FILE=""

release_lock() {
  local status="$1"
  if (( OWNER == 1 )); then
    printf '%s\n' "$status" > "$RESULT_FILE"
    rm -rf "$LOCK_DIR"
    OWNER=0
  fi
}

on_exit() {
  local status=$?
  release_lock "$status"
  exit "$status"
}

trap on_exit EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

while ! mkdir "$LOCK_DIR" 2>/dev/null; do
  OWNER_PID=""
  OWNER_RESULT=""
  if [[ -r "$LOCK_DIR/owner" ]]; then
    IFS=$'\t' read -r OWNER_PID OWNER_RESULT < "$LOCK_DIR/owner" || true
  fi

  if [[ -n "$OWNER_PID" ]] && kill -0 "$OWNER_PID" 2>/dev/null; then
    echo "[agent_preflight] another full preflight is active (pid $OWNER_PID); waiting to coalesce."
    while [[ -d "$LOCK_DIR" ]] && kill -0 "$OWNER_PID" 2>/dev/null; do
      sleep 0.1
    done
    if [[ -n "$OWNER_RESULT" && -f "$OWNER_RESULT" ]]; then
      OWNER_STATUS="$(<"$OWNER_RESULT")"
      if [[ "$OWNER_STATUS" =~ ^[0-9]+$ ]]; then
        echo "[agent_preflight] coalesced with active full preflight (exit $OWNER_STATUS)."
        trap - EXIT
        exit "$OWNER_STATUS"
      fi
    fi
    echo "[agent_preflight] active owner ended without a result; retrying ownership."
    continue
  fi

  echo "[agent_preflight] clearing stale single-flight state."
  rm -rf "$LOCK_DIR"
done

OWNER=1
RESULT_FILE="$STATE_DIR/result.$$.${RANDOM}"
printf '%s\t%s\n' "$$" "$RESULT_FILE" > "$LOCK_DIR/owner"

CHECKS_COMMAND="${WAVECRATE_AGENT_CI_CHECKS_COMMAND:-$ROOT_DIR/scripts/internal/agent/run_agent_ci_checks.sh}"
if [[ ! -x "$CHECKS_COMMAND" ]]; then
  echo "[agent_preflight] ERROR: missing executable full-check command: $CHECKS_COMMAND" >&2
  exit 1
fi

echo "[agent_preflight] full preflight owner: $$"
"$CHECKS_COMMAND"

echo "[agent_preflight] OK"
