#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/run.sh <sandbox|clean|logs|bug-bundle> [args...]
EOF
}

if (( $# == 0 )); then
  usage
  exit 0
fi

command="$1"
shift

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/internal/run"

case "$command" in
  sandbox) exec "$script_dir/run_sandbox.sh" "$@" ;;
  clean) exec "$script_dir/clean_sandbox.sh" "$@" ;;
  logs) exec "$script_dir/latest_log.sh" "$@" ;;
  bug-bundle) exec "$script_dir/bug_bundle.sh" "$@" ;;
  -h|--help) usage ;;
  *)
    echo "Unknown run command: $command" >&2
    usage >&2
    exit 2
    ;;
esac
