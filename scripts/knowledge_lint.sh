#!/usr/bin/env bash

# Developer-doc hygiene checks (diff-aware where appropriate).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BASE_REF=""
HEAD_REF="HEAD"

usage() {
  cat <<'EOF'
Usage: scripts/knowledge_lint.sh [--base <ref>] [--head <ref>]

Runs:
- scripts/check_docs_index.sh
- scripts/check_codeowners_coverage.sh
- scripts/check_markdown_links.sh (diff-aware)
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
      echo "[knowledge_lint] Unknown argument: $1" >&2
      usage >&2
      exit 2 ;;
  esac
done

./scripts/check_docs_index.sh
./scripts/check_codeowners_coverage.sh
./scripts/check_markdown_links.sh ${BASE_REF:+--base "$BASE_REF"} --head "$HEAD_REF"
