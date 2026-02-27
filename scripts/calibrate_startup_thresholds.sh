#!/usr/bin/env bash

# Calibrate startup-first-paint perf thresholds on a compositor-backed host and
# write/update the tracked startup threshold lock file.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

LOCK_OUT="${SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_OUT:-$ROOT_DIR/scripts/perf_locks/startup_thresholds.env}"
RUNS="${SEMPAL_PERF_GUARD_RUNS:-7}"

if [[ -z "${WAYLAND_DISPLAY:-}" && -z "${DISPLAY:-}" ]]; then
  echo "[perf_guard] ERROR: startup calibration requires a compositor-backed host (WAYLAND_DISPLAY or DISPLAY)." >&2
  exit 2
fi

echo "[perf_guard] calibrating startup thresholds (runs=${RUNS})"
SEMPAL_PERF_GUARD_RUNS="$RUNS" \
SEMPAL_PERF_GUARD_STARTUP_PROFILE=1 \
SEMPAL_PERF_GUARD_STARTUP_REQUIRE_VALID_RUNS=1 \
SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_IN="" \
SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_OUT="$LOCK_OUT" \
bash scripts/run_perf_guard.sh

echo "[perf_guard] startup threshold lock refreshed: $LOCK_OUT"
