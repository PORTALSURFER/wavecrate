#!/usr/bin/env bash

# Reports the current line counts of files in the file size budget allowlist.
#
# This is intended for scheduled “entropy” runs and quick local auditing. It
# does not fail the build; it prints a Markdown report to stdout.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

LIMIT=400
ALLOWLIST_PATH="docs/file_size_budget_allowlist.txt"

usage() {
  cat <<'EOF'
Usage: scripts/report_file_size_budget_allowlist.sh [--limit <n>] [--allowlist <path>]

Prints a Markdown report of allowlisted Rust files and their line counts,
highlighting which are still above the limit and which can be removed from the
allowlist.
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
      echo "[allowlist_report] Unknown argument: $1" >&2
      usage >&2
      exit 2 ;;
  esac
done

if [[ ! -f "$ALLOWLIST_PATH" ]]; then
  echo "# File size budget allowlist report"
  echo
  echo "Allowlist file not found: \`$ALLOWLIST_PATH\`"
  exit 0
fi

timestamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

while IFS= read -r line || [[ -n "$line" ]]; do
  [[ -z "$line" ]] && continue
  [[ "$line" == \#* ]] && continue
  file="$line"
  if [[ ! -f "$file" ]]; then
    printf "missing\t0\t%s\n" "$file" >>"$tmp"
    continue
  fi
  count="$(wc -l <"$file" | tr -d '[:space:]')"
  if [[ "$count" -gt "$LIMIT" ]]; then
    printf "over\t%s\t%s\n" "$count" "$file" >>"$tmp"
  else
    printf "ok\t%s\t%s\n" "$count" "$file" >>"$tmp"
  fi
done <"$ALLOWLIST_PATH"

total="$(wc -l <"$tmp" | tr -d '[:space:]')"
over="$(awk -F'\t' '$1=="over"{c++} END{print c+0}' "$tmp")"
ok="$(awk -F'\t' '$1=="ok"{c++} END{print c+0}' "$tmp")"
missing="$(awk -F'\t' '$1=="missing"{c++} END{print c+0}' "$tmp")"

echo "# File size budget allowlist report"
echo
echo "- Timestamp (UTC): \`$timestamp\`"
echo "- Limit: \`$LIMIT\` lines"
echo "- Allowlist: \`$ALLOWLIST_PATH\`"
echo "- Entries: total=$total over=$over ok=$ok missing=$missing"
echo

if (( missing > 0 )); then
  echo "## Missing files (stale allowlist entries)"
  echo
  awk -F'\t' '$1=="missing"{print "- `" $3 "`"}' "$tmp"
  echo
fi

if (( over > 0 )); then
  echo "## Still over budget (prioritized)"
  echo
  echo "| Lines | File |"
  echo "| ---: | --- |"
  awk -F'\t' '$1=="over"{print $2 "\t" $3}' "$tmp" | sort -nr | awk -F'\t' '{print "| " $1 " | `" $2 "` |"}'
  echo
else
  echo "## Still over budget"
  echo
  echo "None."
  echo
fi

if (( ok > 0 )); then
  echo "## Now within budget (can remove from allowlist)"
  echo
  echo "| Lines | File |"
  echo "| ---: | --- |"
  awk -F'\t' '$1=="ok"{print $2 "\t" $3}' "$tmp" | sort -nr | awk -F'\t' '{print "| " $1 " | `" $2 "` |"}'
  echo
fi

