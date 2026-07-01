#!/usr/bin/env bash

# Optional Linux developer-only startup threshold calibration.
#
# Release-risk startup perf evidence is owned by scripts/perf.* guard on
# supported app platforms. This helper remains for contributor threshold refresh
# on Linux compositor-backed hosts; it does not imply shipped Linux app support.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

LOCK_OUT="${WAVECRATE_PERF_GUARD_STARTUP_LOCK_ENV_OUT:-$ROOT_DIR/scripts/internal/perf/locks/startup_thresholds.env}"
RUNS="${WAVECRATE_PERF_GUARD_RUNS:-7}"

if [[ -z "${WAYLAND_DISPLAY:-}" && -z "${DISPLAY:-}" ]]; then
  echo "[perf_guard] ERROR: developer-only startup calibration requires a Linux compositor-backed host (WAYLAND_DISPLAY or DISPLAY)." >&2
  exit 2
fi

echo "[perf_guard] calibrating startup thresholds with Linux developer tooling (runs=${RUNS})"
WAVECRATE_PERF_GUARD_RUNS="$RUNS" \
WAVECRATE_PERF_GUARD_STARTUP_PROFILE=1 \
WAVECRATE_PERF_GUARD_STARTUP_REQUIRE_VALID_RUNS=1 \
WAVECRATE_PERF_GUARD_STARTUP_LOCK_ENV_IN="" \
WAVECRATE_PERF_GUARD_STARTUP_LOCK_ENV_OUT="$LOCK_OUT" \
bash scripts/internal/perf/run_perf_guard.sh

echo "[perf_guard] startup threshold lock refreshed: $LOCK_OUT"
