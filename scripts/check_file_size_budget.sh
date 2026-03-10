#!/usr/bin/env bash

# Enforces a per-file line budget for Rust sources.
#
# By default, the script checks only added/modified Rust files in `src/`, `tests/`,
# and `vendor/radiant/src` relative to a git diff range, plus any staged/unstaged
# working tree changes. Known legacy exceptions live in `docs/file_size_budget_allowlist.txt`.

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

LIMIT=400
BASE_REF=""
HEAD_REF="HEAD"
CHECK_ALL=0
COLLECTED_FILE_COUNT=0
COLLECT_SCOPE=""
TRACKED_PATHS=(src tests vendor/radiant/src)

usage() {
  cat <<'EOF'
Usage: scripts/check_file_size_budget.sh [--base <ref>] [--head <ref>] [--limit <n>] [--all]

Checks Rust files under `src/`, `tests/`, and `vendor/radiant/src` and fails if any
non-allowlisted file exceeds the line budget.

Default behavior:
- If --base/--head are provided: checks files added/modified in that range.
- Also checks staged/unstaged working tree edits.

Options:
  --base <ref>   Git ref/sha for diff base (CI passes this).
  --head <ref>   Git ref/sha for diff head (default: HEAD).
  --limit <n>    Maximum allowed lines (default: 400).
  --all          Check all tracked Rust files in scope (ignores diff).
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --base)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[file_budget] --base requires a value." >&2
        usage >&2
        exit 2
      fi
      BASE_REF="${2:-}"; shift 2 ;;
    --head)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[file_budget] --head requires a value." >&2
        usage >&2
        exit 2
      fi
      HEAD_REF="${2:-}"; shift 2 ;;
    --limit)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[file_budget] --limit requires a numeric value." >&2
        usage >&2
        exit 2
      fi
      if ! [[ "${2:-}" =~ ^[0-9]+$ ]]; then
        echo "[file_budget] --limit must be a positive integer." >&2
        exit 2
      fi
      LIMIT="${2:-}"; shift 2 ;;
    --all)
      CHECK_ALL=1; shift ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "[file_budget] Unknown argument: $1" >&2
      usage >&2
      exit 2 ;;
  esac
done

ALLOWLIST_PATH="$ROOT_DIR/docs/file_size_budget_allowlist.txt"
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

collect_files() {
  local base="$1"
  local head="$2"
  local -a raw_files=()
  local -a rust_files=()

  if (( CHECK_ALL == 1 )); then
    mapfile -t raw_files < <(sempal_git ls-files -- "${TRACKED_PATHS[@]}" || true)
    COLLECT_SCOPE="all"
  elif [[ -n "$base" ]] && git_has_commit "$base" && git_has_commit "$head"; then
    mapfile -t raw_files < <(
      sempal_git diff --name-only --diff-filter=AM "$base...$head" -- "${TRACKED_PATHS[@]}" \
        || true
    )
    COLLECT_SCOPE="diff(base...head)"
  elif git_has_commit "$head"; then
    # If base isn't available (e.g. first push), fall back to the head commit's file list.
    mapfile -t raw_files < <(
      sempal_git show --name-only --pretty=format: "$head" -- "${TRACKED_PATHS[@]}" || true
    )
    COLLECT_SCOPE="diff(head)"
  else
    COLLECT_SCOPE="diff"
    raw_files=()
  fi

  local staged_count=0
  local unstaged_count=0
  if (( CHECK_ALL != 1 )); then
    local -a staged=()
    local -a unstaged=()
    mapfile -t staged < <(
      sempal_git diff --name-only --diff-filter=AM --cached -- "${TRACKED_PATHS[@]}" || true
    )
    mapfile -t unstaged < <(
      sempal_git diff --name-only --diff-filter=AM -- "${TRACKED_PATHS[@]}" || true
    )
    staged_count="${#staged[@]}"
    unstaged_count="${#unstaged[@]}"
    raw_files+=("${staged[@]}" "${unstaged[@]}")
  fi

  local candidate
  for candidate in "${raw_files[@]}"; do
    [[ "$candidate" == src/* ]] && [[ "$candidate" == *.rs ]] && rust_files+=("$candidate")
    [[ "$candidate" == tests/* ]] && [[ "$candidate" == *.rs ]] && rust_files+=("$candidate")
    [[ "$candidate" == vendor/radiant/src/* ]] && [[ "$candidate" == *.rs ]] && rust_files+=("$candidate")
  done

  local -a uniq_files=()
  mapfile -t uniq_files < <(printf "%s\n" "${rust_files[@]}" | sort -u || true)

  COLLECTED_FILE_COUNT="${#uniq_files[@]}"
  echo "[file_budget] collected_file_count=${COLLECTED_FILE_COUNT} (scope=${COLLECT_SCOPE}, raw=${#raw_files[@]}, staged=${staged_count}, unstaged=${unstaged_count})" >&2

  if (( COLLECTED_FILE_COUNT > 0 )); then
    printf "%s\n" "${uniq_files[@]}"
  fi
}

files="$(collect_files "$BASE_REF" "$HEAD_REF")"
if [[ -z "${files:-}" ]]; then
  echo "[file_budget] No changed Rust files detected."
  exit 0
fi

violations=0
checked=0

while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  [[ -f "$file" ]] || continue
  checked=$((checked + 1))

  if [[ -n "${ALLOWLIST[$file]+x}" ]]; then
    continue
  fi

  line_count="$(wc -l <"$file" | tr -d '[:space:]')"
  if [[ "$line_count" -gt "$LIMIT" ]]; then
    if (( violations == 0 )); then
      echo "[file_budget] File size budget violations (limit: $LIMIT lines):" >&2
    fi
    echo " - $file: $line_count" >&2
    violations=$((violations + 1))
  fi
done <<<"$files"

if (( checked == 0 )); then
  echo "[file_budget] No matching Rust files found to check."
  exit 0
fi

if (( violations > 0 )); then
  echo "[file_budget] Fix by splitting files into focused modules, or (temporarily) add to allowlist: $ALLOWLIST_PATH" >&2
  exit 1
fi

echo "[file_budget] OK ($checked files checked)"
exit 0
