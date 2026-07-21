#!/usr/bin/env bash

set -euo pipefail

if (( $# == 0 )); then
  echo "Usage: run_validation_command.sh <command> [args...]" >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

if [[ "$(uname -s)" != "Darwin" ]]; then
  exec "$@"
fi

# shellcheck source=scripts/internal/validation/use_validation_target.sh
source "$ROOT_DIR/scripts/internal/validation/use_validation_target.sh"
wavecrate_use_validation_target "$ROOT_DIR"

if wavecrate_has_enclosing_validation_watchdog; then
  exec "$@"
fi

export WAVECRATE_VALIDATION_WATCHDOG_ACTIVE=1
exec python3 "$ROOT_DIR/scripts/internal/validation/run_with_progress_watchdog.py" "$@"
