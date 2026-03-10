#!/usr/bin/env bash

# Prevents introducing new coupling to the legacy `src/app` module from
# non-legacy codepaths.
#
# The check is diff-aware: it inspects only added lines in diffs for `crate::app`
# usage and ignores existing coupling in unchanged lines.
#
# Scope:
# - Checks diffs under `src/`
# - Skips legacy paths: `src/app/**`, `src/legacy_runtime/**`
# - Allows a small transitional allowlist in `docs/legacy_app_coupling_allowlist.txt`

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ -f "$ROOT_DIR/scripts/git_diff_env.sh" ]]; then
  # shellcheck source=scripts/git_diff_env.sh
  source "$ROOT_DIR/scripts/git_diff_env.sh"
else
  sempal_git() {
    git "$@"
  }
fi

BASE_REF=""
HEAD_REF="HEAD"

usage() {
  cat <<'EOF'
Usage: scripts/check_legacy_app_coupling.sh [--base <ref>] [--head <ref>]

Fails when added lines in `src/**` introduce new `crate::app` references outside
legacy paths (`src/app/**`, `src/legacy_runtime/**`) and outside the allowlist:
  docs/legacy_app_coupling_allowlist.txt
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --base)
      BASE_REF="${2:-}"; shift 2 ;;
    --head)
      HEAD_REF="${2:-}"; shift 2 ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "[legacy_app] Unknown argument: $1" >&2
      usage >&2
      exit 2 ;;
  esac
done

ALLOWLIST_PATH="$ROOT_DIR/docs/legacy_app_coupling_allowlist.txt"
declare -A ALLOWLIST=()
if [[ -f "$ALLOWLIST_PATH" ]]; then
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$line" ]] && continue
    [[ "$line" == \#* ]] && continue
    ALLOWLIST["$line"]=1
  done < "$ALLOWLIST_PATH"
fi

git_has_commit() {
  sempal_git rev-parse --verify --quiet "$1^{commit}" >/dev/null 2>&1
}

is_legacy_path() {
  local file="$1"
  [[ "$file" == src/app/* ]] && return 0
  [[ "$file" == src/legacy_runtime/* ]] && return 0
  return 1
}

is_allowlisted() {
  local file="$1"
  [[ -n "${ALLOWLIST[$file]+x}" ]]
}

scan_diff_stream() {
  local label="$1"
  local violations=0
  local current=""

  while IFS= read -r line; do
    case "$line" in
      "+++ b/"*)
        current="${line#+++ b/}"
        ;;
      "+ "*|"+\t"*|"+")
        # skip (won't happen; keep for clarity)
        ;;
      "+"*)
        # Ignore diff metadata.
        [[ "$line" == "+++"* ]] && continue
        [[ -z "$current" ]] && continue
        [[ "$current" != src/* ]] && continue
        is_legacy_path "$current" && continue
        is_allowlisted "$current" && continue

        if [[ "$line" =~ \bcrate::app\b ]]; then
          if (( violations == 0 )); then
            echo "[legacy_app] New legacy coupling detected ($label):" >&2
            echo "[legacy_app] Do not introduce new crate::app references outside src/app/." >&2
            echo "[legacy_app] If this is a transitional shim, add the file to $ALLOWLIST_PATH." >&2
          fi
          echo " - $current: ${line#+}" >&2
          violations=$((violations + 1))
        fi
        ;;
    esac
  done

  if (( violations > 0 )); then
    return 1
  fi
  return 0
}

scan_git_diff() {
  local label="$1"
  shift
  if ! sempal_git diff --unified=0 --diff-filter=AMR "$@" -- src | scan_diff_stream "$label"; then
    return 1
  fi
  return 0
}

status=0

if [[ -n "$BASE_REF" ]] && git_has_commit "$BASE_REF" && git_has_commit "$HEAD_REF"; then
  scan_git_diff "range $BASE_REF...$HEAD_REF" "$BASE_REF...$HEAD_REF" || status=1
fi

# Always check local changes too.
scan_git_diff "staged" --cached || status=1
scan_git_diff "unstaged" || status=1

if (( status != 0 )); then
  exit 1
fi

echo "[legacy_app] OK"
exit 0
