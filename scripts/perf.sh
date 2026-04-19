#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/perf.sh <guard|calibrate-startup|wheel-stability> [args...]
EOF
}

if (( $# == 0 )); then
  usage
  exit 0
fi

command="$1"
shift

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/internal/perf"

case "$command" in
  guard) exec "$script_dir/run_perf_guard.sh" "$@" ;;
  calibrate-startup) exec "$script_dir/calibrate_startup_thresholds.sh" "$@" ;;
  wheel-stability) exec "$script_dir/run_perf_wheel_stability.sh" "$@" ;;
  -h|--help) usage ;;
  *)
    echo "Unknown perf command: $command" >&2
    usage >&2
    exit 2
    ;;
esac
