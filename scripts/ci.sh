#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/ci.sh <smoke|agent|quick|local> [args...]
EOF
}

if (( $# == 0 )); then
  usage
  exit 0
fi

command="$1"
shift

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/internal/ci"

case "$command" in
  smoke) exec "$script_dir/devcheck.sh" "$@" ;;
  agent) exec "$script_dir/ci_agent.sh" "$@" ;;
  quick) exec "$script_dir/ci_quick.sh" "$@" ;;
  local) exec "$script_dir/ci_local.sh" "$@" ;;
  -h|--help) usage ;;
  *)
    echo "Unknown CI command: $command" >&2
    usage >&2
    exit 2
    ;;
esac
