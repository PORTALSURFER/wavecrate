#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: github_ref_sha.sh [--allow-missing] <owner/repo> <ref>

Prints the commit SHA pointed to by a GitHub git ref, such as tags/nightly.
When --allow-missing is set, a 404 is treated as an empty result.
EOF
}

allow_missing=0
if [[ "${1:-}" == "--allow-missing" ]]; then
  allow_missing=1
  shift
fi

if [[ $# -ne 2 || "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  if [[ $# -eq 0 || "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    exit 0
  fi
  exit 2
fi

repository="$1"
ref="$2"
error_log="$(mktemp)"
trap 'rm -f "$error_log"' EXIT

if sha="$(gh api "repos/${repository}/git/ref/${ref}" --jq '.object.sha' 2>"$error_log")"; then
  printf '%s\n' "$sha"
  exit 0
else
  status=$?
fi

if [[ "$allow_missing" -eq 1 ]] && grep -q "HTTP 404" "$error_log"; then
  exit 0
fi

cat "$error_log" >&2
exit "$status"
