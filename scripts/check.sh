#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/check.sh <next-branch|migration-boundary|file-size-budget|docs-index|codeowners|markdown-links|knowledge> [args...]
EOF
}

if (( $# == 0 )); then
  usage
  exit 0
fi

command="$1"
shift

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/check"

case "$command" in
  next-branch) exec "$script_dir/check_next_branch.sh" "$@" ;;
  migration-boundary) exec "$script_dir/check_migration_boundary.sh" "$@" ;;
  file-size-budget) exec "$script_dir/check_file_size_budget.sh" "$@" ;;
  docs-index) exec "$script_dir/check_docs_index.sh" "$@" ;;
  codeowners) exec "$script_dir/check_codeowners_coverage.sh" "$@" ;;
  markdown-links) exec "$script_dir/check_markdown_links.sh" "$@" ;;
  knowledge) exec "$script_dir/knowledge_lint.sh" "$@" ;;
  -h|--help) usage ;;
  *)
    echo "Unknown check command: $command" >&2
    usage >&2
    exit 2
    ;;
esac
