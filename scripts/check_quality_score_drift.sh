#!/usr/bin/env bash

# Checks quality score drift for high-visibility guardrails.
#
# The guardrail stack should be reflected in `docs/QUALITY_SCORE.md` so regressions
# stay visible to future agents.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BASE_REF=""
HEAD_REF="HEAD"
QUALITY_SCORE_PATH="docs/QUALITY_SCORE.md"
QUALITY_AREA="Agent-facing guardrails"
MIN_HEALTHY_SCORE=4
MAX_DEGRADED_SCORE=3

usage() {
  cat <<'EOF'
Usage: scripts/check_quality_score_drift.sh [--base <ref>] [--head <ref>]

Verifies `docs/QUALITY_SCORE.md` is updated to reflect the current health of
high-visibility shell/Rust guardrails.

Options:
  --base <ref>   Git ref/sha for diff base (CI passes this).
  --head <ref>   Git ref/sha for diff head (default: HEAD).
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --base)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[quality_score] --base requires a value." >&2
        usage >&2
        exit 2
      fi
      BASE_REF="${2:-}"
      shift 2
      ;;
    --head)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[quality_score] --head requires a value." >&2
        usage >&2
        exit 2
      fi
      HEAD_REF="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[quality_score] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

run_check() {
  local label="$1"
  shift

  local status=0
  set +e
  "$@"
  status=$?
  set -e

  if (( status == 0 )); then
    echo "[quality_score] PASS: $label"
    return 0
  fi

  echo "[quality_score] FAIL: $label (exit $status)" >&2
  return "$status"
}

extract_quality_score() {
  local area="$1"
  awk -F'|' -v target="$area" '
    $0 !~ /^\|/ { next }
    /^\|[[:space:]]*Area[[:space:]]*\|/ { next }
    /^\|[[:space:]]*-+/{ next }
    {
      area = $2
      score = $3
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", area)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", score)
      if (area == target) {
        print score
        exit
      }
    }' "$QUALITY_SCORE_PATH"
}

assert_score_in_range() {
  local score="$1"
  local current_state="$2"
  local reason

  if (( score < 0 || score > 5 )); then
    reason="Quality score must be in range 0-5"
    echo "[quality_score] FAIL: $reason" >&2
    return 1
  fi

  if [[ "$current_state" == "healthy" ]]; then
    if (( score < MIN_HEALTHY_SCORE )); then
      echo "[quality_score] FAIL: High-visibility guardrails are passing, but score (${score}) for '$QUALITY_AREA' is below ${MIN_HEALTHY_SCORE}." >&2
      echo "[quality_score] Update this row in ${QUALITY_SCORE_PATH} to reflect the repaired state." >&2
      return 1
    fi
    return 0
  fi

  if (( score > MAX_DEGRADED_SCORE )); then
    echo "[quality_score] FAIL: Guardrails are degraded, but score (${score}) for '$QUALITY_AREA' still appears healthy." >&2
    echo "[quality_score] Lower this row in ${QUALITY_SCORE_PATH} until this is no longer the case." >&2
    return 1
  fi
  return 0
}

script_args=()
if [[ -n "$BASE_REF" ]]; then
  script_args+=(--base "$BASE_REF")
fi
if [[ -n "$HEAD_REF" ]]; then
  script_args+=(--head "$HEAD_REF")
fi

file_size_status=0
taste_status=0

run_check "scripts/check_file_size_budget.sh" ./scripts/check_file_size_budget.sh "${script_args[@]}" || file_size_status=1
run_check "scripts/check_rust_taste_invariants.sh" ./scripts/check_rust_taste_invariants.sh "${script_args[@]}" || taste_status=1

guardrail_failed=0
if (( file_size_status != 0 || taste_status != 0 )); then
  guardrail_failed=1
fi

quality_score="$(extract_quality_score "$QUALITY_AREA" || true)"
if [[ -z "$quality_score" ]]; then
  echo "[quality_score] FAIL: Missing '$QUALITY_AREA' row in ${QUALITY_SCORE_PATH}." >&2
  exit 1
fi

if [[ ! "$quality_score" =~ ^[0-9]$ ]]; then
  echo "[quality_score] FAIL: Parsed quality score '$quality_score' is not a single digit." >&2
  exit 1
fi

if (( quality_score > 5 || quality_score < 0 )); then
  echo "[quality_score] FAIL: Parsed quality score '$quality_score' is outside expected range." >&2
  exit 1
fi

if (( guardrail_failed != 0 )); then
  if ! assert_score_in_range "$quality_score" "degraded"; then
    exit 1
  fi
  echo "[quality_score] NOTICE: Guardrails are currently failing; quality score is downgraded (${quality_score})."
else
  if ! assert_score_in_range "$quality_score" "healthy"; then
    exit 1
  fi
  echo "[quality_score] OK: guardrails are healthy and score is ${quality_score}."
fi

exit 0
