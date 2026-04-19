#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/agent.sh <request|preflight|checks|install-hooks> [args...]
EOF
}

if (( $# == 0 )); then
  usage
  exit 0
fi

command="$1"
shift

case "$command" in
  request)
    exec "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/internal/agent/run_agent_request.sh" "$@"
    ;;
  preflight)
    exec "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/internal/agent/run_agent_preflight.sh" "$@"
    ;;
  checks)
    exec "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/internal/agent/run_agent_ci_checks.sh" "$@"
    ;;
  install-hooks)
    exec "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/internal/agent/install_agent_preflight_hooks.sh" "$@"
    ;;
  -h|--help)
    usage
    ;;
  *)
    echo "Unknown agent command: $command" >&2
    usage >&2
    exit 2
    ;;
esac
