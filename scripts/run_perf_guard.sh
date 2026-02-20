#!/usr/bin/env bash

# Run deterministic runtime interaction benchmarks and emit drift diagnostics.
#
# This guard may fail when explicit fail-threshold env overrides are configured
# (or for scenarios with default fail thresholds). Warning thresholds remain
# non-blocking.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_PATH="${SEMPAL_PERF_GUARD_OUT:-target/perf/bench.json}"
GUI_ROWS="${SEMPAL_PERF_GUARD_GUI_ROWS:-2500}"
GUI_INTERACTION_ROWS="${SEMPAL_PERF_GUARD_GUI_INTERACTION_ROWS:-1500}"
GUI_INTERACTION_ITERS="${SEMPAL_PERF_GUARD_GUI_INTERACTION_ITERS:-24}"
WARMUP_ITERS="${SEMPAL_PERF_GUARD_WARMUP_ITERS:-3}"
MEASURE_ITERS="${SEMPAL_PERF_GUARD_MEASURE_ITERS:-16}"
RUNS="${SEMPAL_PERF_GUARD_RUNS:-1}"
PERF_STATE_ROOT="${SEMPAL_PERF_GUARD_STATE_ROOT:-$ROOT_DIR/target/perf/runtime}"

mkdir -p "$(dirname "$OUT_PATH")"
mkdir -p "$PERF_STATE_ROOT/config" "$PERF_STATE_ROOT/data" "$PERF_STATE_ROOT/state"

if ! [[ "$RUNS" =~ ^[0-9]+$ ]] || [ "$RUNS" -lt 1 ]; then
  echo "[perf_guard] ERROR: SEMPAL_PERF_GUARD_RUNS must be an integer >= 1" >&2
  exit 1
fi

# Keep benchmark config/data writes inside the repo unless the caller overrides XDG dirs.
export XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-$PERF_STATE_ROOT/config}"
export XDG_DATA_HOME="${XDG_DATA_HOME:-$PERF_STATE_ROOT/data}"
export XDG_STATE_HOME="${XDG_STATE_HOME:-$PERF_STATE_ROOT/state}"

declare -a REPORT_PATHS=()
for run in $(seq 1 "$RUNS"); do
  run_out="$OUT_PATH"
  if [ "$RUNS" -gt 1 ]; then
    run_out="${OUT_PATH%.json}.run${run}.json"
  fi
  REPORT_PATHS+=("$run_out")
  echo "[perf_guard] running sempal-bench interaction profile (run ${run}/${RUNS})"
  cargo run --bin sempal-bench -- \
    --out "$run_out" \
    --no-analysis \
    --no-similarity \
    --gui \
    --gui-rows "$GUI_ROWS" \
    --gui-interaction-rows "$GUI_INTERACTION_ROWS" \
    --gui-interaction-iters "$GUI_INTERACTION_ITERS" \
    --warmup-iters "$WARMUP_ITERS" \
    --measure-iters "$MEASURE_ITERS"
done

if [ "$RUNS" -gt 1 ]; then
  last_index=$((${#REPORT_PATHS[@]} - 1))
  cp "${REPORT_PATHS[$last_index]}" "$OUT_PATH"
fi

echo "[perf_guard] parsing benchmark reports (${RUNS} run(s)); canonical report: $OUT_PATH"
python3 - "${REPORT_PATHS[@]}" <<'PY'
import json
import os
import sys
from pathlib import Path
from statistics import median

report_paths = [Path(arg) for arg in sys.argv[1:]]
if not report_paths:
    print("[perf_guard] ERROR: no benchmark reports supplied", file=sys.stderr)
    sys.exit(1)

gui_reports = []
for path in report_paths:
    if not path.exists():
        print(f"[perf_guard] ERROR: report missing at {path}", file=sys.stderr)
        sys.exit(1)
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        print(f"[perf_guard] ERROR: failed to parse benchmark JSON {path}: {exc}", file=sys.stderr)
        sys.exit(1)
    gui = payload.get("gui")
    if not isinstance(gui, dict):
        print(f"[perf_guard] ERROR: missing `gui` benchmark section in {path}", file=sys.stderr)
        sys.exit(1)
    gui_reports.append(gui)

scenarios = [
    (
        "browser_filter_churn_latency",
        "SEMPAL_PERF_WARN_P95_US_FILTER_CHURN",
        10_000,
        "SEMPAL_PERF_FAIL_P95_US_FILTER_CHURN",
        None,
    ),
    (
        "browser_query_churn_latency",
        "SEMPAL_PERF_WARN_P95_US_QUERY_CHURN",
        12_000,
        "SEMPAL_PERF_FAIL_P95_US_QUERY_CHURN",
        None,
    ),
    (
        "browser_sort_toggle_latency",
        "SEMPAL_PERF_WARN_P95_US_SORT_CHURN",
        10_000,
        "SEMPAL_PERF_FAIL_P95_US_SORT_CHURN",
        None,
    ),
    (
        "hover_latency",
        "SEMPAL_PERF_WARN_P95_US_HOVER",
        8_000,
        "SEMPAL_PERF_FAIL_P95_US_HOVER",
        None,
    ),
    (
        "wheel_latency",
        "SEMPAL_PERF_WARN_P95_US_WHEEL",
        10_000,
        "SEMPAL_PERF_FAIL_P95_US_WHEEL",
        30_000,
    ),
    (
        "browser_focus_preview_latency",
        "SEMPAL_PERF_WARN_P95_US_FOCUS_PREVIEW",
        10_000,
        "SEMPAL_PERF_FAIL_P95_US_FOCUS_PREVIEW",
        None,
    ),
    (
        "browser_focus_commit_latency",
        "SEMPAL_PERF_WARN_P95_US_FOCUS_COMMIT",
        16_000,
        "SEMPAL_PERF_FAIL_P95_US_FOCUS_COMMIT",
        100_000,
    ),
    (
        "map_pan_proxy_latency",
        "SEMPAL_PERF_WARN_P95_US_MAP_PAN_PROXY",
        12_000,
        "SEMPAL_PERF_FAIL_P95_US_MAP_PAN_PROXY",
        4_000,
    ),
    (
        "waveform_interaction_latency",
        "SEMPAL_PERF_WARN_P95_US_WAVEFORM",
        10_000,
        "SEMPAL_PERF_FAIL_P95_US_WAVEFORM",
        None,
    ),
    (
        "volume_drag_latency",
        "SEMPAL_PERF_WARN_P95_US_VOLUME",
        8_000,
        "SEMPAL_PERF_FAIL_P95_US_VOLUME",
        None,
    ),
]

stage_names = (
    "input_stage",
    "apply_stage",
    "pull_stage",
    "projection_stage",
)

warned = False
failed = False
contributors = []

for key, warn_env_name, warn_default_limit, fail_env_name, fail_default_limit in scenarios:
    run_summaries = []
    for index, gui in enumerate(gui_reports, start=1):
        summary = gui.get(key)
        if isinstance(summary, dict):
            run_summaries.append(summary)
        else:
            print(
                f"[perf_guard] WARN: missing scenario `{key}` in run {index}; excluding run from this scenario",
                file=sys.stderr,
            )
    if not run_summaries:
        print(
            f"[perf_guard] WARN: skipping scenario `{key}` because no runs provided it",
            file=sys.stderr,
        )
        continue

    p50_values = [int(summary.get("p50_us", 0)) for summary in run_summaries]
    p95_values = [int(summary.get("p95_us", 0)) for summary in run_summaries]
    p99_values = [int(summary.get("p99_us", 0)) for summary in run_summaries]
    max_values = [int(summary.get("max_us", 0)) for summary in run_summaries]
    mean_values = [float(summary.get("mean_us", 0.0)) for summary in run_summaries]
    stddev_values = [float(summary.get("stddev_us", 0.0)) for summary in run_summaries]
    outlier_count_values = [int(summary.get("outlier_high_count", 0)) for summary in run_summaries]
    outlier_ratio_values = [float(summary.get("outlier_high_ratio", 0.0)) for summary in run_summaries]

    p50 = int(median(p50_values))
    p95 = int(median(p95_values))
    p99 = int(median(p99_values))
    max_us = max(max_values) if max_values else 0
    mean_us = float(median(mean_values))
    stddev_us = float(median(stddev_values))
    outlier_high_count = int(median(outlier_count_values))
    outlier_high_ratio = float(median(outlier_ratio_values))
    spread_p95_us = (max(p95_values) - min(p95_values)) if len(p95_values) > 1 else 0

    warn_limit = int(os.getenv(warn_env_name, str(warn_default_limit)))
    fail_limit = None
    fail_raw = os.getenv(fail_env_name)
    if fail_raw is not None:
        fail_limit = int(fail_raw)
    elif fail_default_limit is not None:
        fail_limit = int(fail_default_limit)

    status = f"(warn>{warn_limit}us"
    if fail_limit is not None:
        status += f", fail>{fail_limit}us"
    status += ")"
    print(
        f"[perf_guard] {key}: p50={p50}us p95={p95}us p99={p99}us "
        f"max={max_us}us mean={mean_us:.1f}us stddev={stddev_us:.1f}us "
        f"outliers={outlier_high_count} ({outlier_high_ratio * 100.0:.1f}%) "
        f"runs={len(run_summaries)} p95_spread={spread_p95_us}us {status}"
    )

    stage_reports = []
    for gui in gui_reports:
        attribution = gui.get("interaction_stage_attribution")
        if not isinstance(attribution, dict):
            stage_reports.append(None)
            continue
        stage_reports.append(attribution.get(key))
    if any(isinstance(stage, dict) for stage in stage_reports):
        if not all(isinstance(stage, dict) for stage in stage_reports):
            print(
                f"[perf_guard] WARN: {key} stage attribution missing for one or more runs",
                file=sys.stderr,
            )
        else:
            stage_p95 = {}
            missing_stage_field = False
            for stage_name in stage_names:
                values = []
                for stage in stage_reports:
                    summary = stage.get(stage_name)
                    if not isinstance(summary, dict):
                        missing_stage_field = True
                        break
                    values.append(int(summary.get("p95_us", 0)))
                if missing_stage_field:
                    break
                stage_p95[stage_name] = int(median(values))
            if missing_stage_field:
                print(
                    f"[perf_guard] WARN: {key} stage attribution is missing one or more stage summaries",
                    file=sys.stderr,
                )
            else:
                print(
                    f"[perf_guard]   {key} stage_p95_us: "
                    f"input={stage_p95['input_stage']} "
                    f"apply={stage_p95['apply_stage']} "
                    f"pull={stage_p95['pull_stage']} "
                    f"projection={stage_p95['projection_stage']}"
                )
    segment_reports = []
    for gui in gui_reports:
        attribution = gui.get("interaction_segment_attribution")
        if not isinstance(attribution, dict):
            segment_reports.append(None)
            continue
        segment_reports.append(attribution)
    if any(isinstance(segment, dict) for segment in segment_reports):
        if not all(isinstance(segment, dict) for segment in segment_reports):
            print(
                f"[perf_guard] WARN: {key} segment attribution missing for one or more runs",
                file=sys.stderr,
            )
        else:
            segment_name_by_scenario = {
                "interactive_projection": "status_bar",
                "browser_filter_churn_latency": "browser_rows_window",
                "browser_query_churn_latency": "browser_rows_window",
                "browser_sort_toggle_latency": "browser_rows_window",
                "hover_latency": "browser_frame",
                "wheel_latency": "browser_rows_window",
                "browser_focus_preview_latency": "browser_frame",
                "browser_focus_commit_latency": "browser_rows_window",
                "map_pan_proxy_latency": "map_panel",
                "waveform_interaction_latency": "waveform_overlay",
                "volume_drag_latency": "status_bar",
            }
            segment_name = segment_name_by_scenario.get(key)
            if segment_name is not None:
                values = []
                missing_segment_field = False
                for segment in segment_reports:
                    summary = segment.get(segment_name)
                    if not isinstance(summary, dict):
                        missing_segment_field = True
                        break
                    values.append(
                        (
                            int(summary.get("hit_count", 0)),
                            int(summary.get("miss_count", 0)),
                            int(summary.get("p95_us", 0)),
                        )
                    )
                if missing_segment_field:
                    print(
                        f"[perf_guard] WARN: {key} segment attribution is missing segment `{segment_name}`",
                        file=sys.stderr,
                    )
                else:
                    hits = int(median([value[0] for value in values]))
                    misses = int(median([value[1] for value in values]))
                    segment_p95 = int(median([value[2] for value in values]))
                    print(
                        f"[perf_guard]   {key} segment[{segment_name}] "
                        f"hit={hits} miss={misses} p95={segment_p95}us"
                    )

    if p95 > warn_limit:
        warned = True
        over_ratio = p95 / max(warn_limit, 1)
        contributors.append((over_ratio, key, p95, warn_limit))
        print(
            f"[perf_guard] WARN: {key} median p95 {p95}us exceeded warning limit {warn_limit}us",
            file=sys.stderr,
        )
    if fail_limit is not None and p95 > fail_limit:
        failed = True
        print(
            f"[perf_guard] ERROR: {key} median p95 {p95}us exceeded fail limit {fail_limit}us",
            file=sys.stderr,
        )

if contributors:
    contributors.sort(reverse=True)
    print("[perf_guard] top warning contributors (by p95/limit):")
    for over_ratio, key, p95, limit in contributors[:3]:
        print(
            f"[perf_guard]   - {key}: p95={p95}us limit={limit}us "
            f"ratio={over_ratio:.2f}x"
        )

if warned:
    print("[perf_guard] WARN: latency drift detected (warn-only mode)")
else:
    print("[perf_guard] OK: all scenario p95 values within warning limits")
if failed:
    sys.exit(2)
PY
