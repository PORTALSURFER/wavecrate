#!/usr/bin/env bash

# Prevents introducing new dependencies from `src/app_core` into legacy/UI runtime layers.
#
# Rationale:
# - `app_core` is intended to remain backend-neutral glue and domain projection logic.
# - UI runtime and host glue belong in `src/gui_app` and `src/gui_runtime`.
# - Legacy runtime compatibility belongs in `src/legacy_runtime`.
#
# The check is diff-aware: it inspects only added lines in diffs so existing
# legacy coupling does not block introducing the rule.

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
Usage: scripts/check_app_core_dependency_boundary.sh [--base <ref>] [--head <ref>]

Fails when added lines in `src/app_core/**` introduce any of:
- `crate::legacy_runtime::`
- `crate::gui_app::`
- `crate::gui_runtime::`

Allowlist file (last resort):
  docs/app_core_dependency_boundary_allowlist.txt
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
      echo "[app_core_boundary] Unknown argument: $1" >&2
      usage >&2
      exit 2 ;;
  esac
done

ALLOWLIST_PATH="$ROOT_DIR/docs/app_core_dependency_boundary_allowlist.txt"
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
      "+"*)
        [[ "$line" == "+++"* ]] && continue
        [[ -z "$current" ]] && continue
        [[ "$current" == src/app_core/* ]] || continue
        [[ "$current" == *.rs ]] || continue
        is_allowlisted "$current" && continue

        # Ignore pure comment additions.
        if [[ "$line" =~ ^\\+\\s*// ]]; then
          continue
        fi

        text="${line#+}"
        if [[ "$text" =~ \\bcrate::legacy_runtime:: ]]; then
          if (( violations == 0 )); then
            echo "[app_core_boundary] Violations detected ($label):" >&2
            echo "[app_core_boundary] app_core must not take new dependencies on legacy/UI runtime layers." >&2
            echo "[app_core_boundary] Move code to src/gui_app, src/gui_runtime, or src/legacy_runtime as appropriate." >&2
            echo "[app_core_boundary] Allowlist (last resort): $ALLOWLIST_PATH" >&2
          fi
          echo " - $current: legacy_runtime: $text" >&2
          violations=$((violations + 1))
        fi
        if [[ "$text" =~ \\bcrate::gui_app:: ]]; then
          if (( violations == 0 )); then
            echo "[app_core_boundary] Violations detected ($label):" >&2
            echo "[app_core_boundary] app_core must not take new dependencies on legacy/UI runtime layers." >&2
            echo "[app_core_boundary] Move code to src/gui_app, src/gui_runtime, or src/legacy_runtime as appropriate." >&2
            echo "[app_core_boundary] Allowlist (last resort): $ALLOWLIST_PATH" >&2
          fi
          echo " - $current: gui_app: $text" >&2
          violations=$((violations + 1))
        fi
        if [[ "$text" =~ \\bcrate::gui_runtime:: ]]; then
          if (( violations == 0 )); then
            echo "[app_core_boundary] Violations detected ($label):" >&2
            echo "[app_core_boundary] app_core must not take new dependencies on legacy/UI runtime layers." >&2
            echo "[app_core_boundary] Move code to src/gui_app, src/gui_runtime, or src/legacy_runtime as appropriate." >&2
            echo "[app_core_boundary] Allowlist (last resort): $ALLOWLIST_PATH" >&2
          fi
          echo " - $current: gui_runtime: $text" >&2
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
  sempal_git diff --unified=0 --diff-filter=AMR "$@" -- src/app_core \
    | scan_diff_stream "$label"
}

status=0
if [[ -n "$BASE_REF" ]] && git_has_commit "$BASE_REF" && git_has_commit "$HEAD_REF"; then
  scan_git_diff "range $BASE_REF...$HEAD_REF" "$BASE_REF...$HEAD_REF" || status=1
fi

scan_git_diff "staged" --cached || status=1
scan_git_diff "unstaged" || status=1

if (( status != 0 )); then
  exit 1
fi

echo "[app_core_boundary] OK"
exit 0
