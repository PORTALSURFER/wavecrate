#!/usr/bin/env bash

# Prunes entries from the file size budget allowlist that no longer need to be
# allowlisted (now within budget) or that are stale (missing files).
#
# This is intended for “entropy” automation and small cleanup PRs.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

LIMIT=400
ALLOWLIST_PATH="docs/file_size_budget_allowlist.txt"

usage() {
  cat <<'EOF'
Usage: scripts/prune_file_size_budget_allowlist.sh [--limit <n>] [--allowlist <path>]

Rewrites the allowlist file in-place, removing entries whose file is:
- missing, or
- <= <limit> lines.

Comment and blank lines are preserved.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --limit)
      LIMIT="${2:-}"; shift 2 ;;
    --allowlist)
      ALLOWLIST_PATH="${2:-}"; shift 2 ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "[prune_file_size_budget_allowlist] Unknown argument: $1" >&2
      usage >&2
      exit 2 ;;
  esac
done

if [[ ! -f "$ALLOWLIST_PATH" ]]; then
  echo "[prune_file_size_budget_allowlist] allowlist not found: $ALLOWLIST_PATH" >&2
  exit 0
fi

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

removed_ok=0
removed_missing=0
kept=0

while IFS= read -r line || [[ -n "$line" ]]; do
  if [[ -z "$line" || "$line" == \#* ]]; then
    printf "%s\n" "$line" >>"$tmp"
    continue
  fi

  file="$line"
  if [[ ! -f "$file" ]]; then
    removed_missing=$((removed_missing + 1))
    continue
  fi

  count="$(wc -l <"$file" | tr -d '[:space:]')"
  if [[ "$count" -le "$LIMIT" ]]; then
    removed_ok=$((removed_ok + 1))
    continue
  fi

  kept=$((kept + 1))
  printf "%s\n" "$line" >>"$tmp"
done <"$ALLOWLIST_PATH"

if cmp -s "$tmp" "$ALLOWLIST_PATH"; then
  echo "[prune_file_size_budget_allowlist] no changes (kept=$kept removed_ok=$removed_ok removed_missing=$removed_missing)" >&2
  exit 0
fi

mv "$tmp" "$ALLOWLIST_PATH"
trap - EXIT

echo "[prune_file_size_budget_allowlist] updated $ALLOWLIST_PATH (kept=$kept removed_ok=$removed_ok removed_missing=$removed_missing)" >&2

