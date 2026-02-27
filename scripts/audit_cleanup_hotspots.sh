#!/usr/bin/env bash

# Generates a deterministic cleanup-hotspot snapshot for planning.
#
# The report includes:
# - largest Rust files (line count)
# - files still over the file-size budget limit
# - dead-code and clippy::too_many_arguments suppression density
# - likely test-gap hotspots (large files without local test modules)
#
# Output defaults to `tmp/cleanup_audit_hotspots.md`.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUTPUT_PATH="tmp/cleanup_audit_hotspots.md"
TOP_FILES=20
TOP_SUPPRESSIONS=20
TEST_GAP_MIN_LINES=200
FILE_SIZE_LIMIT=400

usage() {
  cat <<'USAGE'
Usage: scripts/audit_cleanup_hotspots.sh [--output <path>] [--top-files <n>] [--top-suppressions <n>] [--test-gap-min-lines <n>] [--file-size-limit <n>]

Generate a deterministic markdown snapshot of cleanup hotspots.

Options:
  --output <path>            Output markdown path (default: tmp/cleanup_audit_hotspots.md)
  --top-files <n>            Number of largest files to show (default: 20)
  --top-suppressions <n>     Number of files to show for each suppression table (default: 20)
  --test-gap-min-lines <n>   Minimum file size for test-gap heuristic (default: 200)
  --file-size-limit <n>      File-size budget threshold in lines (default: 400)
  -h, --help                 Show help
USAGE
}

is_non_negative_integer() {
  [[ "$1" =~ ^[0-9]+$ ]]
}

while (( $# > 0 )); do
  case "$1" in
    --output)
      OUTPUT_PATH="${2:-}"
      shift 2
      ;;
    --top-files)
      TOP_FILES="${2:-}"
      shift 2
      ;;
    --top-suppressions)
      TOP_SUPPRESSIONS="${2:-}"
      shift 2
      ;;
    --test-gap-min-lines)
      TEST_GAP_MIN_LINES="${2:-}"
      shift 2
      ;;
    --file-size-limit)
      FILE_SIZE_LIMIT="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[cleanup_audit] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

for value_name in TOP_FILES TOP_SUPPRESSIONS TEST_GAP_MIN_LINES FILE_SIZE_LIMIT; do
  value="${!value_name}"
  if ! is_non_negative_integer "$value"; then
    echo "[cleanup_audit] $value_name must be a non-negative integer (got: $value)" >&2
    exit 2
  fi
done

if [[ -z "$OUTPUT_PATH" ]]; then
  echo "[cleanup_audit] --output requires a non-empty path" >&2
  exit 2
fi

mkdir -p "$(dirname "$OUTPUT_PATH")"

tmp_line_counts="$(mktemp)"
tmp_over_limit="$(mktemp)"
tmp_dead_hits="$(mktemp)"
tmp_tma_hits="$(mktemp)"
tmp_dead_counts="$(mktemp)"
tmp_tma_counts="$(mktemp)"
tmp_test_gaps="$(mktemp)"

cleanup() {
  rm -f \
    "$tmp_line_counts" \
    "$tmp_over_limit" \
    "$tmp_dead_hits" \
    "$tmp_tma_hits" \
    "$tmp_dead_counts" \
    "$tmp_tma_counts" \
    "$tmp_test_gaps"
}
trap cleanup EXIT

mapfile -t rust_files < <(git ls-files '*.rs' | LC_ALL=C sort)

for file in "${rust_files[@]}"; do
  if [[ ! -f "$file" ]]; then
    continue
  fi
  line_count="$(wc -l <"$file" | tr -d '[:space:]')"
  printf "%s\t%s\n" "$line_count" "$file" >>"$tmp_line_counts"
  if (( line_count > FILE_SIZE_LIMIT )); then
    printf "%s\t%s\n" "$line_count" "$file" >>"$tmp_over_limit"
  fi

done

rg -n --no-heading --glob '*.rs' '^\s*#\s*\[allow\([^]]*dead_code[^]]*\)\]' >"$tmp_dead_hits" || true
if [[ -s "$tmp_dead_hits" ]]; then
  awk -F: '{counts[$1]++} END {for (file in counts) printf "%d\t%s\n", counts[file], file}' "$tmp_dead_hits" \
    | LC_ALL=C sort -t$'\t' -k1,1nr -k2,2 >"$tmp_dead_counts"
fi

rg -n --no-heading --glob '*.rs' '^\s*#\s*\[allow\([^]]*clippy::too_many_arguments[^]]*\)\]' >"$tmp_tma_hits" || true
if [[ -s "$tmp_tma_hits" ]]; then
  awk -F: '{counts[$1]++} END {for (file in counts) printf "%d\t%s\n", counts[file], file}' "$tmp_tma_hits" \
    | LC_ALL=C sort -t$'\t' -k1,1nr -k2,2 >"$tmp_tma_counts"
fi

while IFS=$'\t' read -r line_count file; do
  if (( line_count < TEST_GAP_MIN_LINES )); then
    continue
  fi
  if [[ "$file" == */tests/* || "$file" == tests/* || "$file" == *_test.rs ]]; then
    continue
  fi
  if rg -q '^\s*#\s*\[cfg\(test\)\]|^\s*mod\s+tests\b' "$file"; then
    continue
  fi
  printf "%s\t%s\n" "$line_count" "$file" >>"$tmp_test_gaps"
done < <(LC_ALL=C sort -t$'\t' -k1,1nr -k2,2 "$tmp_line_counts")

timestamp_utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
git_branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
git_commit="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
total_rust_files="$(wc -l <"$tmp_line_counts" | tr -d '[:space:]')"
over_budget_count="$(wc -l <"$tmp_over_limit" | tr -d '[:space:]')"
dead_supp_files="$(wc -l <"$tmp_dead_counts" | tr -d '[:space:]')"
tma_supp_files="$(wc -l <"$tmp_tma_counts" | tr -d '[:space:]')"
test_gap_count="$(wc -l <"$tmp_test_gaps" | tr -d '[:space:]')"

{
  echo "# Cleanup Hotspot Audit Snapshot"
  echo
  echo "- Generated (UTC): \`$timestamp_utc\`"
  echo "- Branch: \`$git_branch\`"
  echo "- Commit: \`$git_commit\`"
  echo "- Rust files scanned: $total_rust_files"
  echo "- File-size budget limit: \`$FILE_SIZE_LIMIT\`"
  echo
  echo "## Summary"
  echo
  echo "- Over file-size budget: $over_budget_count"
  echo "- Files with \`dead_code\` suppressions: $dead_supp_files"
  echo "- Files with \`clippy::too_many_arguments\` suppressions: $tma_supp_files"
  echo "- Likely large-file test-gap hotspots (heuristic): $test_gap_count"
  echo

  echo "## Largest Rust files"
  echo
  echo "| Lines | File |"
  echo "| ---: | --- |"
  head -n "$TOP_FILES" "$tmp_line_counts" \
    | LC_ALL=C sort -t$'\t' -k1,1nr -k2,2 \
    | while IFS=$'\t' read -r line_count file; do
        echo "| $line_count | \`$file\` |"
      done
  echo

  echo "## Over file-size budget"
  echo
  if (( over_budget_count == 0 )); then
    echo "None."
  else
    echo "| Lines | File |"
    echo "| ---: | --- |"
    LC_ALL=C sort -t$'\t' -k1,1nr -k2,2 "$tmp_over_limit" \
      | while IFS=$'\t' read -r line_count file; do
          echo "| $line_count | \`$file\` |"
        done
  fi
  echo

  echo "## dead_code suppression density"
  echo
  if (( dead_supp_files == 0 )); then
    echo "None."
  else
    echo "| Occurrences | File |"
    echo "| ---: | --- |"
    head -n "$TOP_SUPPRESSIONS" "$tmp_dead_counts" \
      | while IFS=$'\t' read -r count file; do
          echo "| $count | \`$file\` |"
        done
  fi
  echo

  echo "## too_many_arguments suppression density"
  echo
  if (( tma_supp_files == 0 )); then
    echo "None."
  else
    echo "| Occurrences | File |"
    echo "| ---: | --- |"
    head -n "$TOP_SUPPRESSIONS" "$tmp_tma_counts" \
      | while IFS=$'\t' read -r count file; do
          echo "| $count | \`$file\` |"
        done
  fi
  echo

  echo "## Likely test-gap hotspots (heuristic)"
  echo
  echo "Files with at least \`$TEST_GAP_MIN_LINES\` lines and no local \`#[cfg(test)]\` or \`mod tests\` marker."
  echo
  if (( test_gap_count == 0 )); then
    echo "None."
  else
    echo "| Lines | File |"
    echo "| ---: | --- |"
    head -n "$TOP_FILES" "$tmp_test_gaps" \
      | while IFS=$'\t' read -r line_count file; do
          echo "| $line_count | \`$file\` |"
        done
  fi
  echo

  echo "## Suggested follow-up"
  echo
  echo "1. Triage top over-budget files and plan behavior-preserving splits."
  echo "2. Remove or test-gate high-density suppressions after each refactor slice."
  echo "3. Add focused tests for top heuristic gaps where behavior is non-trivial."
} >"$OUTPUT_PATH"

echo "[cleanup_audit] wrote $OUTPUT_PATH"
