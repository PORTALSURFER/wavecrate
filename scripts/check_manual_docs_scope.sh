#!/usr/bin/env bash

# Enforces that `manual/` only contains user-facing docs and site assets.
#
# The check is diff-aware: it fails only when added/modified files under `manual/`
# are outside the allowlist. Deletions are allowed.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BASE_REF=""
HEAD_REF="HEAD"

usage() {
  cat <<'EOF'
Usage: scripts/check_manual_docs_scope.sh [--base <ref>] [--head <ref>]

Fails when added/modified files under `manual/` are outside the allowlist:
  manual/index.md
  manual/usage.md
  manual/design_principles.md
  manual/_config.yml
  manual/_layouts/**
  manual/assets/**
  manual/README.md
  manual/<redirect-stubs>.md (developer docs moved to `docs/`)

The script checks:
- git diff between --base and --head (when provided and resolvable)
- staged changes
- unstaged changes
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
      echo "[manual_scope] Unknown argument: $1" >&2
      usage >&2
      exit 2 ;;
  esac
done

git_has_commit() {
  git rev-parse --verify --quiet "$1^{commit}" >/dev/null 2>&1
}

collect_manual_changes() {
  local base="$1"
  local head="$2"

  local out=()
  if [[ -n "$base" ]] && git_has_commit "$base" && git_has_commit "$head"; then
    mapfile -t out < <(git diff --name-only --diff-filter=AM "$base...$head" -- manual || true)
  elif git_has_commit "$head"; then
    mapfile -t out < <(git show --name-only --pretty=format: "$head" -- manual || true)
  fi

  mapfile -t staged < <(git diff --name-only --diff-filter=AM --cached -- manual || true)
  mapfile -t unstaged < <(git diff --name-only --diff-filter=AM -- manual || true)

  printf "%s\n" "${out[@]}" "${staged[@]}" "${unstaged[@]}" \
    | sed 's#^\\./##' \
    | sort -u || true
}

is_allowlisted() {
  local path="$1"
  case "$path" in
    manual/index.md) return 0 ;;
    manual/usage.md) return 0 ;;
    manual/design_principles.md) return 0 ;;
    manual/_config.yml) return 0 ;;
    manual/README.md) return 0 ;;
    manual/_layouts/*) return 0 ;;
    manual/assets/*) return 0 ;;
    manual/ann_index_container.md) return 0 ;;
    manual/drag_audit.md) return 0 ;;
    manual/feature_vector.md) return 0 ;;
    manual/gui_migration_parity.md) return 0 ;;
    manual/hints.md) return 0 ;;
    manual/icon_assets.md) return 0 ;;
    manual/native_shell_legacy_baseline.md) return 0 ;;
    manual/performance_qa.md) return 0 ;;
    manual/plan.md) return 0 ;;
    manual/styleguide.md) return 0 ;;
    manual/todo.md) return 0 ;;
    manual/transient_audit.md) return 0 ;;
    manual/transient_plan.md) return 0 ;;
    manual/updater-contract.md) return 0 ;;
    *) return 1 ;;
  esac
}

paths="$(collect_manual_changes "$BASE_REF" "$HEAD_REF")"
if [[ -z "${paths:-}" ]]; then
  echo "[manual_scope] No added/modified files detected under manual/."
  exit 0
fi

violations=0
while IFS= read -r path; do
  [[ -z "$path" ]] && continue
  if ! is_allowlisted "$path"; then
    if (( violations == 0 )); then
      echo "[manual_scope] Disallowed added/modified file(s) under manual/:" >&2
      echo "[manual_scope] manual/ is user-facing only; developer docs belong in docs/." >&2
      echo "[manual_scope] Allowlisted paths:" >&2
      echo " - manual/index.md" >&2
      echo " - manual/usage.md" >&2
      echo " - manual/_config.yml" >&2
      echo " - manual/_layouts/**" >&2
      echo " - manual/assets/**" >&2
      echo " - manual/README.md" >&2
    fi
    echo " - $path" >&2
    violations=$((violations + 1))
  fi
done <<<"$paths"

if (( violations > 0 )); then
  exit 1
fi

echo "[manual_scope] OK"
exit 0
