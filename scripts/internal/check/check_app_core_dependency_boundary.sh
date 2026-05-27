#!/usr/bin/env bash

# Prevents introducing new dependencies from `src/app_core` into UI runtime layers.
#
# Rationale:
# - `app_core` is intended to remain backend-neutral glue and domain projection logic.
# - `crate::gui_runtime::` is the live runtime adapter layer for host glue.
# - `crate::gui_app::` should not leak into new `app_core` code.
#
# The check is diff-aware: it inspects only added lines in diffs so existing
# legacy coupling does not block introducing the rule.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"
if [[ -f "$ROOT_DIR/scripts/internal/git_diff_env.sh" ]]; then
  # shellcheck source=scripts/internal/git_diff_env.sh
  source "$ROOT_DIR/scripts/internal/git_diff_env.sh"
else
  wavecrate_git() {
    git "$@"
  }
fi

BASE_REF=""
HEAD_REF="HEAD"

usage() {
  cat <<'EOF'
Usage: scripts/internal/check/check_app_core_dependency_boundary.sh [--base <ref>] [--head <ref>]

Fails when added lines in `src/app_core/**` introduce any of:
- `crate::gui_app::`
- `crate::gui_runtime::`

Allowlist file (last resort):
  scripts/internal/check/allowlists/app_core_dependency_boundary_allowlist.txt
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

ALLOWLIST_PATH="$ROOT_DIR/scripts/internal/check/allowlists/app_core_dependency_boundary_allowlist.txt"

git_has_commit() {
  wavecrate_git rev-parse --verify --quiet "$1^{commit}" >/dev/null 2>&1
}

is_allowlisted() {
  local file="$1"
  local line
  [[ -f "$ALLOWLIST_PATH" ]] || return 1
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$line" ]] && continue
    [[ "$line" == \#* ]] && continue
    [[ "$line" == "$file" ]] && return 0
  done < "$ALLOWLIST_PATH"
  return 1
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
        if [[ "$text" =~ \\bcrate::gui_app:: ]]; then
          if (( violations == 0 )); then
            echo "[app_core_boundary] Violations detected ($label):" >&2
            echo "[app_core_boundary] app_core must not take new dependencies on UI runtime layers." >&2
            echo "[app_core_boundary] Move code into the current runtime or adapter layer (usually src/gui_runtime or the legacy src/app boundary), or invert the dependency." >&2
            echo "[app_core_boundary] Allowlist (last resort): $ALLOWLIST_PATH" >&2
          fi
          echo " - $current: gui_app: $text" >&2
          violations=$((violations + 1))
        fi
        if [[ "$text" =~ \\bcrate::gui_runtime:: ]]; then
          if (( violations == 0 )); then
            echo "[app_core_boundary] Violations detected ($label):" >&2
            echo "[app_core_boundary] app_core must not take new dependencies on UI runtime layers." >&2
            echo "[app_core_boundary] Move code into the current runtime or adapter layer (usually src/gui_runtime or the legacy src/app boundary), or invert the dependency." >&2
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
  wavecrate_git diff --unified=0 --diff-filter=AMR "$@" -- src/app_core \
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
