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
  smoke) script="$script_dir/devcheck.sh" ;;
  agent) script="$script_dir/ci_agent.sh" ;;
  quick) script="$script_dir/ci_quick.sh" ;;
  local) script="$script_dir/ci_local.sh" ;;
  -h|--help)
    usage
    exit 0
    ;;
  *)
    echo "Unknown CI command: $command" >&2
    usage >&2
    exit 2
    ;;
esac

exec "$script_dir/../validation/run_validation_command.sh" "$script" "$@"
