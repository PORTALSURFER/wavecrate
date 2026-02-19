#!/usr/bin/env bash

# Run deterministic runtime interaction benchmarks and emit a warning-only summary.
#
# This guard never fails on latency threshold drift. It only fails when benchmark
# execution or report parsing fails.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_PATH="${SEMPAL_PERF_GUARD_OUT:-target/perf/bench.json}"
GUI_ROWS="${SEMPAL_PERF_GUARD_GUI_ROWS:-2500}"
GUI_INTERACTION_ROWS="${SEMPAL_PERF_GUARD_GUI_INTERACTION_ROWS:-1500}"
GUI_INTERACTION_ITERS="${SEMPAL_PERF_GUARD_GUI_INTERACTION_ITERS:-24}"
WARMUP_ITERS="${SEMPAL_PERF_GUARD_WARMUP_ITERS:-3}"
MEASURE_ITERS="${SEMPAL_PERF_GUARD_MEASURE_ITERS:-16}"
PERF_STATE_ROOT="${SEMPAL_PERF_GUARD_STATE_ROOT:-$ROOT_DIR/target/perf/runtime}"

mkdir -p "$(dirname "$OUT_PATH")"
mkdir -p "$PERF_STATE_ROOT/config" "$PERF_STATE_ROOT/data" "$PERF_STATE_ROOT/state"

# Keep benchmark config/data writes inside the repo unless the caller overrides XDG dirs.
export XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-$PERF_STATE_ROOT/config}"
export XDG_DATA_HOME="${XDG_DATA_HOME:-$PERF_STATE_ROOT/data}"
export XDG_STATE_HOME="${XDG_STATE_HOME:-$PERF_STATE_ROOT/state}"

echo "[perf_guard] running sempal-bench interaction profile"
cargo run --bin sempal-bench -- \
  --out "$OUT_PATH" \
  --no-analysis \
  --no-similarity \
  --gui \
  --gui-rows "$GUI_ROWS" \
  --gui-interaction-rows "$GUI_INTERACTION_ROWS" \
  --gui-interaction-iters "$GUI_INTERACTION_ITERS" \
  --warmup-iters "$WARMUP_ITERS" \
  --measure-iters "$MEASURE_ITERS"

echo "[perf_guard] parsing benchmark report: $OUT_PATH"
python3 - "$OUT_PATH" <<'PY'
import json
import os
import sys
from pathlib import Path

path = Path(sys.argv[1])
if not path.exists():
    print(f"[perf_guard] ERROR: report missing at {path}", file=sys.stderr)
    sys.exit(1)

try:
    payload = json.loads(path.read_text(encoding="utf-8"))
except Exception as exc:
    print(f"[perf_guard] ERROR: failed to parse benchmark JSON: {exc}", file=sys.stderr)
    sys.exit(1)

gui = payload.get("gui")
if not isinstance(gui, dict):
    print("[perf_guard] ERROR: missing `gui` benchmark section", file=sys.stderr)
    sys.exit(1)

scenarios = [
    ("hover_latency", "SEMPAL_PERF_WARN_P95_US_HOVER", 8_000),
    ("wheel_latency", "SEMPAL_PERF_WARN_P95_US_WHEEL", 10_000),
    ("map_pan_proxy_latency", "SEMPAL_PERF_WARN_P95_US_MAP_PAN_PROXY", 12_000),
    ("waveform_interaction_latency", "SEMPAL_PERF_WARN_P95_US_WAVEFORM", 10_000),
]

warned = False
for key, env_name, default_limit in scenarios:
    summary = gui.get(key)
    if not isinstance(summary, dict):
        print(f"[perf_guard] ERROR: missing scenario `{key}`", file=sys.stderr)
        sys.exit(1)
    p50 = int(summary.get("p50_us", 0))
    p95 = int(summary.get("p95_us", 0))
    max_us = int(summary.get("max_us", 0))
    mean_us = float(summary.get("mean_us", 0.0))
    limit = int(os.getenv(env_name, str(default_limit)))
    print(
        f"[perf_guard] {key}: p50={p50}us p95={p95}us max={max_us}us mean={mean_us:.1f}us (warn>{limit}us)"
    )
    if p95 > limit:
        warned = True
        print(
            f"[perf_guard] WARN: {key} p95 {p95}us exceeded warning limit {limit}us",
            file=sys.stderr,
        )

if warned:
    print("[perf_guard] WARN: latency drift detected (warn-only mode)")
else:
    print("[perf_guard] OK: all scenario p95 values within warning limits")
PY
