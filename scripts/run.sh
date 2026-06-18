#!/usr/bin/env bash

set -euo pipefail

usage() {
  local entrypoint="${WAVECRATE_RUN_ENTRYPOINT:-scripts/run.sh}"
  cat <<EOF
Usage: ${entrypoint} <sandbox|clean|logs|bug-bundle> [args...]

Commands:
  sandbox      Run Wavecrate with an isolated sandbox profile.
  clean        Delete the repo-local sandbox profile.
  logs         Print the newest resolved Wavecrate log.
  bug-bundle   Create a small diagnostic bundle.
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
  -h|--help|-Help|help) usage ;;
  *)
    echo "Unknown run command: $command" >&2
    usage >&2
    exit 2
    ;;
esac
