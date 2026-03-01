#!/usr/bin/env bash

# Run deterministic runtime interaction benchmarks and emit drift diagnostics.
#
# This guard may fail when explicit fail-threshold env overrides are configured
# (or for scenarios with default fail thresholds). Warning thresholds remain
# non-blocking.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/setup_headless_audio.sh
source "$ROOT_DIR/scripts/setup_headless_audio.sh"
sempal_setup_headless_audio "perf_guard"

RADIANT_RUNTIME_FILE="$ROOT_DIR/vendor/radiant/src/gui_runtime/native_vello.rs"
RADIANT_NESTED_RUNTIME_FILE="$ROOT_DIR/vendor/radiant/vendor/radiant/src/gui_runtime/native_vello.rs"

if [ ! -f "$RADIANT_RUNTIME_FILE" ]; then
  echo "[perf_guard] runtime internals missing; initializing git submodules recursively"
  git submodule update --init --recursive vendor/radiant >/dev/null
fi

if [ ! -f "$RADIANT_RUNTIME_FILE" ]; then
  if [ -f "$RADIANT_NESTED_RUNTIME_FILE" ]; then
    echo "[perf_guard] ERROR: detected nested radiant checkout at vendor/radiant/vendor/radiant; expected top-level vendor/radiant crate" >&2
    echo "[perf_guard] ERROR: repin vendor/radiant to a commit where vendor/radiant/Cargo.toml has name = \"radiant\"" >&2
  else
    echo "[perf_guard] ERROR: missing runtime internals at $RADIANT_RUNTIME_FILE" >&2
  fi
  exit 1
fi

OUT_PATH="${SEMPAL_PERF_GUARD_OUT:-target/perf/bench.json}"
GUI_ROWS="${SEMPAL_PERF_GUARD_GUI_ROWS:-2500}"
GUI_INTERACTION_ROWS="${SEMPAL_PERF_GUARD_GUI_INTERACTION_ROWS:-1500}"
GUI_INTERACTION_ITERS="${SEMPAL_PERF_GUARD_GUI_INTERACTION_ITERS:-24}"
WARMUP_ITERS="${SEMPAL_PERF_GUARD_WARMUP_ITERS:-3}"
MEASURE_ITERS="${SEMPAL_PERF_GUARD_MEASURE_ITERS:-16}"
RUNS="${SEMPAL_PERF_GUARD_RUNS:-1}"
PERF_STATE_ROOT="${SEMPAL_PERF_GUARD_STATE_ROOT:-$ROOT_DIR/target/perf/runtime}"
STARTUP_PROFILE_RAW="${SEMPAL_PERF_GUARD_STARTUP_PROFILE:-0}"
STARTUP_TIMEOUT_SECS="${SEMPAL_PERF_GUARD_STARTUP_TIMEOUT_SECS:-6}"
STARTUP_REQUIRE_VALID_RAW="${SEMPAL_PERF_GUARD_STARTUP_REQUIRE_VALID_RUNS:-0}"
STARTUP_LOCK_ENV_OUT="${SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_OUT:-}"
STARTUP_LOCK_ENV_IN="${SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_IN:-$ROOT_DIR/scripts/perf_locks/startup_thresholds.env}"
FRAME_QUALITY_LOCK_ENV_OUT="${SEMPAL_PERF_GUARD_FRAME_QUALITY_LOCK_ENV_OUT:-}"

startup_profile_enabled=0
case "${STARTUP_PROFILE_RAW,,}" in
  1|true|yes|on)
    startup_profile_enabled=1
    ;;
esac

startup_require_valid_runs=0
case "${STARTUP_REQUIRE_VALID_RAW,,}" in
  1|true|yes|on)
    startup_require_valid_runs=1
    ;;
esac

load_threshold_lock_env() {
  local lock_path="$1"
  local lock_label="$2"
  if [[ -z "$lock_path" ]]; then
    return
  fi
  if [[ ! -f "$lock_path" ]]; then
    return
  fi
  # shellcheck disable=SC1090
  source "$lock_path"
  echo "[perf_guard] loaded ${lock_label} threshold lock env: $lock_path"
}

load_threshold_lock_env "$STARTUP_LOCK_ENV_IN" "startup"

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
declare -a STARTUP_LOG_PATHS=()

if (( startup_profile_enabled == 1 )) && ! command -v timeout >/dev/null 2>&1; then
  echo "[perf_guard] WARN: startup profiling requested but \`timeout\` is unavailable; skipping startup capture" >&2
  startup_profile_enabled=0
fi

if (( startup_profile_enabled == 1 )); then
  startup_binary="${ROOT_DIR}/target/debug/sempal"
  echo "[perf_guard] building sempal startup binary for profile capture"
  cargo build --bin sempal >/dev/null
  if [[ "$RUNS" -ge 3 ]]; then
    startup_min_valid_runs_default=3
  else
    startup_min_valid_runs_default=1
  fi
  STARTUP_MIN_VALID_RUNS="${SEMPAL_PERF_GUARD_STARTUP_MIN_VALID_RUNS:-$startup_min_valid_runs_default}"
fi

if [[ "$RUNS" -ge 3 ]]; then
  frame_quality_min_runs_default=3
else
  frame_quality_min_runs_default=1
fi
FRAME_QUALITY_LOCK_MIN_RUNS="${SEMPAL_PERF_GUARD_FRAME_QUALITY_LOCK_MIN_RUNS:-$frame_quality_min_runs_default}"

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
  if (( startup_profile_enabled == 1 )); then
    startup_log="${OUT_PATH%.json}.startup.run${run}.log"
    STARTUP_LOG_PATHS+=("$startup_log")
    echo "[perf_guard] capturing native startup profile (run ${run}/${RUNS})"
    set +e
    SEMPAL_NATIVE_STARTUP_PROFILE=1 \
      timeout --signal=TERM --kill-after=1s "${STARTUP_TIMEOUT_SECS}s" \
      "$startup_binary" >"$startup_log" 2>&1
    startup_status=$?
    set -e
    if [[ "$startup_status" -ne 0 && "$startup_status" -ne 124 && "$startup_status" -ne 143 ]]; then
      echo "[perf_guard] WARN: startup profiling exited with status ${startup_status}; see ${startup_log}" >&2
    fi
  fi
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
        "waveform_pan_zoom_adjacent_latency",
        "SEMPAL_PERF_WARN_P95_US_WAVEFORM_ADJACENT",
        12_000,
        "SEMPAL_PERF_FAIL_P95_US_WAVEFORM_ADJACENT",
        None,
    ),
    (
        "volume_drag_latency",
        "SEMPAL_PERF_WARN_P95_US_VOLUME",
        8_000,
        "SEMPAL_PERF_FAIL_P95_US_VOLUME",
        None,
    ),
    (
        "idle_cursor_motion_latency",
        "SEMPAL_PERF_WARN_P95_US_IDLE_CURSOR",
        8_000,
        "SEMPAL_PERF_FAIL_P95_US_IDLE_CURSOR",
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
jank_contributors = []
warn_jank_ratio = float(os.getenv("SEMPAL_PERF_WARN_FRAME_JANK_RATIO", "0.10"))
warn_missed_present_ratio = float(
    os.getenv("SEMPAL_PERF_WARN_MISSED_PRESENT_PROXY_RATIO", "0.05")
)
fail_jank_ratio_raw = os.getenv("SEMPAL_PERF_FAIL_FRAME_JANK_RATIO")
fail_jank_ratio = float(fail_jank_ratio_raw) if fail_jank_ratio_raw is not None else None
fail_missed_present_ratio_raw = os.getenv("SEMPAL_PERF_FAIL_MISSED_PRESENT_PROXY_RATIO")
fail_missed_present_ratio = (
    float(fail_missed_present_ratio_raw)
    if fail_missed_present_ratio_raw is not None
    else None
)

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
    frame_budget_values = [int(summary.get("frame_budget_us", 0)) for summary in run_summaries]
    frame_jank_count_values = [int(summary.get("frame_jank_count", 0)) for summary in run_summaries]
    frame_jank_ratio_values = [float(summary.get("frame_jank_ratio", 0.0)) for summary in run_summaries]
    missed_present_count_values = [
        int(summary.get("missed_present_proxy_count", 0)) for summary in run_summaries
    ]
    missed_present_ratio_values = [
        float(summary.get("missed_present_proxy_ratio", 0.0))
        for summary in run_summaries
    ]

    p50 = int(median(p50_values))
    p95 = int(median(p95_values))
    p99 = int(median(p99_values))
    max_us = max(max_values) if max_values else 0
    mean_us = float(median(mean_values))
    stddev_us = float(median(stddev_values))
    outlier_high_count = int(median(outlier_count_values))
    outlier_high_ratio = float(median(outlier_ratio_values))
    spread_p95_us = (max(p95_values) - min(p95_values)) if len(p95_values) > 1 else 0
    frame_budget_us = int(median(frame_budget_values))
    frame_jank_count = int(median(frame_jank_count_values))
    frame_jank_ratio = float(median(frame_jank_ratio_values))
    missed_present_count = int(median(missed_present_count_values))
    missed_present_ratio = float(median(missed_present_ratio_values))

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
    print(
        f"[perf_guard]   {key} frame_quality_proxy: budget={frame_budget_us}us "
        f"jank={frame_jank_count} ({frame_jank_ratio * 100.0:.1f}%) "
        f"missed_present={missed_present_count} ({missed_present_ratio * 100.0:.1f}%) "
        f"(warn_jank>{warn_jank_ratio * 100.0:.1f}% warn_missed>{warn_missed_present_ratio * 100.0:.1f}%)"
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
                "waveform_pan_zoom_adjacent_latency": "waveform_overlay",
                "volume_drag_latency": "status_bar",
                "idle_cursor_motion_latency": "waveform_overlay",
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
    rebuild_reports = []
    for gui in gui_reports:
        attribution = gui.get("interaction_rebuild_cause_attribution")
        if not isinstance(attribution, dict):
            rebuild_reports.append(None)
            continue
        rebuild_reports.append(attribution.get(key))
    if any(isinstance(rebuild, dict) for rebuild in rebuild_reports):
        if not all(isinstance(rebuild, dict) for rebuild in rebuild_reports):
            print(
                f"[perf_guard] WARN: {key} rebuild-cause attribution missing for one or more runs",
                file=sys.stderr,
            )
        else:
            values = []
            missing_rebuild_field = False
            for rebuild in rebuild_reports:
                explicit = rebuild.get("explicit_static_rebuild_count")
                dirty_mask = rebuild.get("dirty_mask_static_rebuild_count")
                model_pull = rebuild.get("bridge_model_pull_rebuild_count")
                motion_pull = rebuild.get("bridge_motion_pull_rebuild_count")
                waveform_motion_pull = rebuild.get("waveform_motion_pull_rebuild_count", 0)
                chrome_motion_pull = rebuild.get("chrome_motion_pull_rebuild_count", 0)
                if not all(
                    isinstance(value, int)
                    for value in (
                        explicit,
                        dirty_mask,
                        model_pull,
                        motion_pull,
                        waveform_motion_pull,
                        chrome_motion_pull,
                    )
                ):
                    missing_rebuild_field = True
                    break
                values.append(
                    (
                        explicit,
                        dirty_mask,
                        model_pull,
                        motion_pull,
                        waveform_motion_pull,
                        chrome_motion_pull,
                    )
                )
            if missing_rebuild_field:
                print(
                    f"[perf_guard] WARN: {key} rebuild-cause attribution has missing counters",
                    file=sys.stderr,
                )
            else:
                explicit = int(median([value[0] for value in values]))
                dirty_mask = int(median([value[1] for value in values]))
                model_pull = int(median([value[2] for value in values]))
                motion_pull = int(median([value[3] for value in values]))
                waveform_motion_pull = int(median([value[4] for value in values]))
                chrome_motion_pull = int(median([value[5] for value in values]))
                print(
                    f"[perf_guard]   {key} rebuild_causes: "
                    f"explicit_static={explicit} dirty_mask_static={dirty_mask} "
                    f"model_pull={model_pull} motion_pull={motion_pull} "
                    f"waveform_motion_pull={waveform_motion_pull} "
                    f"chrome_motion_pull={chrome_motion_pull}"
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
    if frame_jank_ratio > warn_jank_ratio:
        warned = True
        over_ratio = frame_jank_ratio / max(warn_jank_ratio, 1e-9)
        jank_contributors.append(
            (
                over_ratio,
                key,
                "jank_ratio",
                frame_jank_ratio * 100.0,
                warn_jank_ratio * 100.0,
            )
        )
        print(
            f"[perf_guard] WARN: {key} median frame_jank_ratio "
            f"{frame_jank_ratio * 100.0:.1f}% exceeded warning limit "
            f"{warn_jank_ratio * 100.0:.1f}%",
            file=sys.stderr,
        )
    if fail_jank_ratio is not None and frame_jank_ratio > fail_jank_ratio:
        failed = True
        print(
            f"[perf_guard] ERROR: {key} median frame_jank_ratio "
            f"{frame_jank_ratio * 100.0:.1f}% exceeded fail limit "
            f"{fail_jank_ratio * 100.0:.1f}%",
            file=sys.stderr,
        )
    if missed_present_ratio > warn_missed_present_ratio:
        warned = True
        over_ratio = missed_present_ratio / max(warn_missed_present_ratio, 1e-9)
        jank_contributors.append(
            (
                over_ratio,
                key,
                "missed_present_ratio",
                missed_present_ratio * 100.0,
                warn_missed_present_ratio * 100.0,
            )
        )
        print(
            f"[perf_guard] WARN: {key} median missed_present_proxy_ratio "
            f"{missed_present_ratio * 100.0:.1f}% exceeded warning limit "
            f"{warn_missed_present_ratio * 100.0:.1f}%",
            file=sys.stderr,
        )
    if (
        fail_missed_present_ratio is not None
        and missed_present_ratio > fail_missed_present_ratio
    ):
        failed = True
        print(
            f"[perf_guard] ERROR: {key} median missed_present_proxy_ratio "
            f"{missed_present_ratio * 100.0:.1f}% exceeded fail limit "
            f"{fail_missed_present_ratio * 100.0:.1f}%",
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
if jank_contributors:
    jank_contributors.sort(reverse=True)
    print("[perf_guard] top frame-quality warning contributors:")
    for over_ratio, key, metric_name, value_pct, limit_pct in jank_contributors[:3]:
        print(
            f"[perf_guard]   - {key} {metric_name}: value={value_pct:.1f}% "
            f"limit={limit_pct:.1f}% ratio={over_ratio:.2f}x"
        )

if warned:
    print("[perf_guard] WARN: latency drift detected (warn-only mode)")
else:
    print("[perf_guard] OK: all scenario p95 values within warning limits")
if failed:
    sys.exit(2)
PY

if [[ -n "$FRAME_QUALITY_LOCK_ENV_OUT" ]]; then
  python3 scripts/perf_frame_quality_lock_thresholds.py \
    --out "$FRAME_QUALITY_LOCK_ENV_OUT" \
    --min-runs "$FRAME_QUALITY_LOCK_MIN_RUNS" \
    "${REPORT_PATHS[@]}"
fi

if (( startup_profile_enabled == 1 )); then
  startup_summary_out="${SEMPAL_PERF_GUARD_STARTUP_SUMMARY_OUT:-${OUT_PATH%.json}.startup_summary.json}"
  startup_summary_cmd=(
    python3
    scripts/perf_startup_summary.py
    --output
    "$startup_summary_out"
    --warn-first-present-ms
    "${SEMPAL_PERF_WARN_STARTUP_FIRST_PRESENT_MS:-800}"
    --min-valid-runs
    "$STARTUP_MIN_VALID_RUNS"
  )
  if [[ -n "${SEMPAL_PERF_FAIL_STARTUP_FIRST_PRESENT_MS:-}" ]]; then
    startup_summary_cmd+=(
      --fail-first-present-ms
      "$SEMPAL_PERF_FAIL_STARTUP_FIRST_PRESENT_MS"
    )
  fi
  if [[ -n "${SEMPAL_PERF_WARN_STARTUP_FIRST_PRESENT_SPREAD_MS:-}" ]]; then
    startup_summary_cmd+=(
      --warn-first-present-spread-ms
      "$SEMPAL_PERF_WARN_STARTUP_FIRST_PRESENT_SPREAD_MS"
    )
  fi
  if [[ -n "${SEMPAL_PERF_FAIL_STARTUP_FIRST_PRESENT_SPREAD_MS:-}" ]]; then
    startup_summary_cmd+=(
      --fail-first-present-spread-ms
      "$SEMPAL_PERF_FAIL_STARTUP_FIRST_PRESENT_SPREAD_MS"
    )
  fi
  if (( startup_require_valid_runs == 1 )); then
    startup_summary_cmd+=(--require-min-valid-runs)
  fi
  startup_summary_cmd+=("${STARTUP_LOG_PATHS[@]}")
  "${startup_summary_cmd[@]}"
  if [[ -n "$STARTUP_LOCK_ENV_OUT" ]]; then
    STARTUP_LOCK_MIN_VALID_RUNS="${SEMPAL_PERF_GUARD_STARTUP_LOCK_MIN_VALID_RUNS:-$STARTUP_MIN_VALID_RUNS}"
    python3 scripts/perf_startup_lock_thresholds.py \
      --summary "$startup_summary_out" \
      --out "$STARTUP_LOCK_ENV_OUT" \
      --min-valid-runs "$STARTUP_LOCK_MIN_VALID_RUNS"
  fi
fi
