#!/usr/bin/env bash

# Collect and evaluate wheel-latency stability evidence for perf-guard threshold promotion.
#
# Modes:
#   - collect: run perf guard windows and persist benchmark artifacts
#   - evaluate: analyze collected artifacts and emit readiness summary JSON
#   - collect-and-evaluate (default): run both steps in sequence

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="${1:-collect-and-evaluate}"
if [[ "$MODE" != "collect" && "$MODE" != "evaluate" && "$MODE" != "collect-and-evaluate" ]]; then
  echo "[wheel_stability] ERROR: mode must be collect, evaluate, or collect-and-evaluate" >&2
  exit 2
fi

STATE_ROOT="${SEMPAL_PERF_WHEEL_STABILITY_ROOT:-$ROOT_DIR/target/perf/wheel_stability}"
WINDOWS="${SEMPAL_PERF_WHEEL_STABILITY_WINDOWS:-7}"
RUNS_PER_WINDOW="${SEMPAL_PERF_WHEEL_STABILITY_RUNS_PER_WINDOW:-3}"
REQUIRED_WINDOWS="${SEMPAL_PERF_WHEEL_STABILITY_REQUIRED_WINDOWS:-7}"
TARGET_P95_US="${SEMPAL_PERF_WHEEL_STABILITY_TARGET_P95_US:-16000}"
MAX_P95_SPREAD_US="${SEMPAL_PERF_WHEEL_STABILITY_MAX_P95_SPREAD_US:-7000}"
MAX_STDDEV_US="${SEMPAL_PERF_WHEEL_STABILITY_MAX_STDDEV_US:-6000}"
MAX_OUTLIER_RATIO="${SEMPAL_PERF_WHEEL_STABILITY_MAX_OUTLIER_RATIO:-0.35}"
ENFORCE_READY="${SEMPAL_PERF_WHEEL_STABILITY_ENFORCE_READY:-0}"
SUMMARY_PATH="${SEMPAL_PERF_WHEEL_STABILITY_SUMMARY_OUT:-$STATE_ROOT/wheel_stability_summary.json}"

require_int() {
  local name="$1"
  local value="$2"
  if ! [[ "$value" =~ ^[0-9]+$ ]]; then
    echo "[wheel_stability] ERROR: $name must be an integer" >&2
    exit 2
  fi
}

require_int "SEMPAL_PERF_WHEEL_STABILITY_WINDOWS" "$WINDOWS"
require_int "SEMPAL_PERF_WHEEL_STABILITY_RUNS_PER_WINDOW" "$RUNS_PER_WINDOW"
require_int "SEMPAL_PERF_WHEEL_STABILITY_REQUIRED_WINDOWS" "$REQUIRED_WINDOWS"
require_int "SEMPAL_PERF_WHEEL_STABILITY_TARGET_P95_US" "$TARGET_P95_US"
require_int "SEMPAL_PERF_WHEEL_STABILITY_MAX_P95_SPREAD_US" "$MAX_P95_SPREAD_US"
require_int "SEMPAL_PERF_WHEEL_STABILITY_MAX_STDDEV_US" "$MAX_STDDEV_US"

mkdir -p "$STATE_ROOT"
mkdir -p "$(dirname "$SUMMARY_PATH")"

if [[ "$MODE" == "collect" || "$MODE" == "collect-and-evaluate" ]]; then
  if [ "$WINDOWS" -lt 1 ]; then
    echo "[wheel_stability] ERROR: SEMPAL_PERF_WHEEL_STABILITY_WINDOWS must be >= 1" >&2
    exit 2
  fi
  if [ "$RUNS_PER_WINDOW" -lt 1 ]; then
    echo "[wheel_stability] ERROR: SEMPAL_PERF_WHEEL_STABILITY_RUNS_PER_WINDOW must be >= 1" >&2
    exit 2
  fi

  for window in $(seq 1 "$WINDOWS"); do
    ts="$(date -u +"%Y%m%dT%H%M%SZ")"
    out_path="$STATE_ROOT/wheel_window_${ts}_${window}.json"
    echo "[wheel_stability] collecting window ${window}/${WINDOWS}: $out_path (runs=$RUNS_PER_WINDOW)"
    SEMPAL_PERF_GUARD_OUT="$out_path" \
    SEMPAL_PERF_GUARD_RUNS="$RUNS_PER_WINDOW" \
      bash scripts/run_perf_guard.sh
  done
fi

if [[ "$MODE" == "evaluate" || "$MODE" == "collect-and-evaluate" ]]; then
  python3 - "$STATE_ROOT" "$SUMMARY_PATH" "$REQUIRED_WINDOWS" "$TARGET_P95_US" \
    "$MAX_P95_SPREAD_US" "$MAX_STDDEV_US" "$MAX_OUTLIER_RATIO" "$ENFORCE_READY" <<'PY'
import json
import re
import sys
from pathlib import Path
from statistics import median

state_root = Path(sys.argv[1])
summary_path = Path(sys.argv[2])
required_windows = int(sys.argv[3])
target_p95_us = int(sys.argv[4])
max_p95_spread_us = int(sys.argv[5])
max_stddev_us = int(sys.argv[6])
max_outlier_ratio = float(sys.argv[7])
enforce_ready = sys.argv[8].strip().lower() in {"1", "true", "yes", "on"}

window_pattern = re.compile(r"wheel_window_(\d{8}T\d{6}Z)_([0-9]+)\.json$")


def parse_wheel_summary(report_path: Path) -> dict:
    payload = json.loads(report_path.read_text(encoding="utf-8"))
    gui = payload.get("gui")
    if not isinstance(gui, dict):
        raise ValueError(f"missing gui section in {report_path}")
    wheel = gui.get("wheel_latency")
    if not isinstance(wheel, dict):
        raise ValueError(f"missing wheel_latency in {report_path}")
    return wheel


def collect_run_files(base_report: Path) -> list[Path]:
    stem = base_report.stem
    run_files = sorted(base_report.parent.glob(f"{stem}.run*.json"))
    if run_files:
        return run_files
    return [base_report]


window_reports = []
for report in sorted(state_root.glob("wheel_window_*.json")):
    match = window_pattern.search(report.name)
    if not match:
        continue
    ts = match.group(1)
    ordinal = int(match.group(2))
    run_files = collect_run_files(report)

    p95_values = []
    p99_values = []
    stddev_values = []
    outlier_ratios = []
    for run_file in run_files:
        wheel = parse_wheel_summary(run_file)
        p95_values.append(int(wheel.get("p95_us", 0)))
        p99_values.append(int(wheel.get("p99_us", 0)))
        stddev_values.append(float(wheel.get("stddev_us", 0.0)))
        outlier_ratios.append(float(wheel.get("outlier_high_ratio", 0.0)))

    if not p95_values:
        continue

    window_reports.append(
        {
            "timestamp": ts,
            "ordinal": ordinal,
            "report": str(report),
            "run_count": len(p95_values),
            "median_p95_us": int(median(p95_values)),
            "median_p99_us": int(median(p99_values)),
            "median_stddev_us": float(median(stddev_values)),
            "median_outlier_ratio": float(median(outlier_ratios)),
            "p95_spread_us": int(max(p95_values) - min(p95_values)) if len(p95_values) > 1 else 0,
        }
    )

window_reports.sort(key=lambda item: (item["timestamp"], item["ordinal"]))
selected = window_reports[-required_windows:] if required_windows > 0 else []

reasons = []
if len(selected) < required_windows:
    reasons.append(
        f"need {required_windows} windows, found {len(selected)}"
    )

for report in selected:
    if report["median_p95_us"] > target_p95_us:
        reasons.append(
            f"{Path(report['report']).name}: median_p95_us={report['median_p95_us']} > target={target_p95_us}"
        )
    if report["p95_spread_us"] > max_p95_spread_us:
        reasons.append(
            f"{Path(report['report']).name}: p95_spread_us={report['p95_spread_us']} > max={max_p95_spread_us}"
        )
    if report["median_stddev_us"] > max_stddev_us:
        reasons.append(
            f"{Path(report['report']).name}: median_stddev_us={report['median_stddev_us']:.1f} > max={max_stddev_us}"
        )
    if report["median_outlier_ratio"] > max_outlier_ratio:
        reasons.append(
            f"{Path(report['report']).name}: median_outlier_ratio={report['median_outlier_ratio']:.3f} > max={max_outlier_ratio:.3f}"
        )

ready = len(reasons) == 0
summary = {
    "ready_for_fail_promotion": ready,
    "required_windows": required_windows,
    "evaluated_windows": len(selected),
    "criteria": {
        "target_p95_us": target_p95_us,
        "max_p95_spread_us": max_p95_spread_us,
        "max_stddev_us": max_stddev_us,
        "max_outlier_ratio": max_outlier_ratio,
    },
    "selected_windows": selected,
    "reasons": reasons,
}

summary_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")

print(
    "[wheel_stability] "
    f"evaluated_windows={len(selected)} required_windows={required_windows} "
    f"ready_for_fail_promotion={'true' if ready else 'false'}"
)
print(f"[wheel_stability] summary: {summary_path}")
if reasons:
    print("[wheel_stability] reasons:")
    for reason in reasons:
        print(f"[wheel_stability]   - {reason}")

if enforce_ready and not ready:
    sys.exit(2)
PY
fi
