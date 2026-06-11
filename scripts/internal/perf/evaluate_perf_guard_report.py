#!/usr/bin/env python3
"""Evaluate perf guard benchmark reports against the shared validation contract."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from statistics import median
from typing import Any


STAGE_NAMES = ("input_stage", "apply_stage", "pull_stage", "projection_stage")

SEGMENT_NAME_BY_SCENARIO = {
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


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Evaluate Wavecrate perf-guard benchmark reports."
    )
    parser.add_argument(
        "--contract",
        default="scripts/internal/data/validation_contract.json",
        help="Shared validation contract JSON path.",
    )
    parser.add_argument("reports", nargs="+", help="Benchmark report JSON files.")
    return parser.parse_args()


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        raise ValueError(f"failed to parse JSON {path}: {exc}") from exc


def load_gui_reports(report_paths: list[Path]) -> list[dict[str, Any]]:
    reports = []
    for path in report_paths:
        if not path.exists():
            raise ValueError(f"report missing at {path}")
        payload = load_json(path)
        gui = payload.get("gui")
        if not isinstance(gui, dict):
            raise ValueError(f"missing `gui` benchmark section in {path}")
        reports.append(gui)
    return reports


def median_int(values: list[int | float]) -> int:
    return int(round(median(values)))


def required_number(summary: dict[str, Any], key: str) -> int | float:
    value = summary.get(key)
    if not isinstance(value, (int, float)):
        raise ValueError(f"missing numeric `{key}` in benchmark report")
    return value


def optional_float_env(name: str) -> float | None:
    raw = os.getenv(name)
    if raw is None or raw.strip() == "":
        return None
    return float(raw)


def warn(message: str) -> None:
    print(f"[perf_guard] WARN: {message}", file=sys.stderr)


def emit_projection_diagnostics(gui_reports: list[dict[str, Any]]) -> None:
    retained = [
        gui.get("retained_app_model_projection_p95_us")
        for gui in gui_reports
        if isinstance(gui.get("retained_app_model_projection_p95_us"), (int, float))
    ]
    if retained:
        p95 = median_int(retained)
        print(
            "[perf_guard] retained_app_model_projection_p95_us: "
            f"median={p95} us (diagnostic, retained runtime path)"
        )

    controller = []
    for gui in gui_reports:
        summary = gui.get("controller_app_model_projection")
        if isinstance(summary, dict) and isinstance(summary.get("p95_us"), (int, float)):
            controller.append(int(summary["p95_us"]))
    if controller:
        p95 = median_int(controller)
        print(
            "[perf_guard] controller_app_model_projection_p95_us: "
            f"median={p95} us (diagnostic, legacy controller path)"
        )


def scenario_samples(
    gui_reports: list[dict[str, Any]], scenario_key: str
) -> list[dict[str, Any]]:
    samples = []
    for index, gui in enumerate(gui_reports, start=1):
        summary = gui.get(scenario_key)
        if isinstance(summary, dict):
            samples.append(summary)
        else:
            warn(f"missing scenario `{scenario_key}` in run {index}; excluding run")
    if not samples:
        warn(f"skipping scenario `{scenario_key}` because no runs provided it")
    return samples


def emit_stage_attribution(
    key: str, gui_reports: list[dict[str, Any]], run_count: int
) -> None:
    stage_reports = []
    for gui in gui_reports:
        attribution = gui.get("interaction_stage_attribution")
        stage_reports.append(attribution.get(key) if isinstance(attribution, dict) else None)
    if not any(isinstance(stage, dict) for stage in stage_reports):
        return
    if not all(isinstance(stage, dict) for stage in stage_reports):
        warn(f"{key} stage attribution missing for one or more runs")
        return

    stage_p95 = {}
    for stage_name in STAGE_NAMES:
        values = []
        for stage in stage_reports:
            summary = stage.get(stage_name)
            if not isinstance(summary, dict):
                warn(f"{key} stage attribution is missing one or more stage summaries")
                return
            values.append(int(summary.get("p95_us", 0)))
        stage_p95[stage_name] = median_int(values)
    print(
        f"[perf_guard]   {key} stage_p95_us: "
        f"input={stage_p95['input_stage']} "
        f"apply={stage_p95['apply_stage']} "
        f"pull={stage_p95['pull_stage']} "
        f"projection={stage_p95['projection_stage']}"
    )


def emit_segment_attribution(key: str, gui_reports: list[dict[str, Any]]) -> None:
    segment_name = SEGMENT_NAME_BY_SCENARIO.get(key)
    if segment_name is None:
        return
    segment_reports = [
        gui.get("interaction_segment_attribution")
        for gui in gui_reports
        if isinstance(gui.get("interaction_segment_attribution"), dict)
    ]
    if not segment_reports:
        return
    if len(segment_reports) != len(gui_reports):
        warn(f"{key} segment attribution missing for one or more runs")
        return

    values = []
    for segment in segment_reports:
        summary = segment.get(segment_name)
        if not isinstance(summary, dict):
            warn(f"{key} segment attribution is missing segment `{segment_name}`")
            return
        values.append(
            (
                int(summary.get("hit_count", 0)),
                int(summary.get("miss_count", 0)),
                int(summary.get("p95_us", 0)),
            )
        )
    hits = median_int([value[0] for value in values])
    misses = median_int([value[1] for value in values])
    p95 = median_int([value[2] for value in values])
    print(
        f"[perf_guard]   {key} segment[{segment_name}] "
        f"hit={hits} miss={misses} p95={p95}us"
    )


def emit_rebuild_attribution(key: str, gui_reports: list[dict[str, Any]]) -> None:
    rebuild_reports = []
    for gui in gui_reports:
        attribution = gui.get("interaction_rebuild_cause_attribution")
        rebuild_reports.append(attribution.get(key) if isinstance(attribution, dict) else None)
    if not any(isinstance(rebuild, dict) for rebuild in rebuild_reports):
        return
    if not all(isinstance(rebuild, dict) for rebuild in rebuild_reports):
        warn(f"{key} rebuild-cause attribution missing for one or more runs")
        return

    values = []
    for rebuild in rebuild_reports:
        required = (
            rebuild.get("explicit_static_rebuild_count"),
            rebuild.get("dirty_mask_static_rebuild_count"),
            rebuild.get("bridge_model_pull_rebuild_count"),
            rebuild.get("bridge_motion_pull_rebuild_count"),
            rebuild.get("waveform_motion_pull_rebuild_count", 0),
            rebuild.get("chrome_motion_pull_rebuild_count", 0),
        )
        if not all(isinstance(value, int) for value in required):
            warn(f"{key} rebuild-cause attribution has missing counters")
            return
        values.append(required)

    medians = [median_int([value[index] for value in values]) for index in range(6)]
    print(
        f"[perf_guard]   {key} rebuild_causes: "
        f"explicit_static={medians[0]} dirty_mask_static={medians[1]} "
        f"model_pull={medians[2]} motion_pull={medians[3]} "
        f"waveform_motion_pull={medians[4]} chrome_motion_pull={medians[5]}"
    )


def evaluate_scenarios(
    contract: dict[str, Any], gui_reports: list[dict[str, Any]]
) -> tuple[bool, bool]:
    frame_quality = contract["perf"]["frame_quality"]
    warn_jank_ratio = float(
        os.getenv(
            frame_quality["warn_jank_env"],
            str(frame_quality["warn_jank_default"]),
        )
    )
    warn_missed_ratio = float(
        os.getenv(
            frame_quality["warn_missed_present_env"],
            str(frame_quality["warn_missed_present_default"]),
        )
    )
    fail_jank_ratio = optional_float_env(frame_quality["fail_jank_env"])
    fail_missed_ratio = optional_float_env(frame_quality["fail_missed_present_env"])
    warned = False
    failed = False
    contributors = []
    jank_contributors = []

    for scenario in contract["perf"]["scenarios"]:
        key = scenario["key"]
        samples = scenario_samples(gui_reports, key)
        if not samples:
            continue

        p50 = median_int([required_number(sample, "p50_us") for sample in samples])
        p95_values = [int(required_number(sample, "p95_us")) for sample in samples]
        p95 = median_int(p95_values)
        p99 = median_int([required_number(sample, "p99_us") for sample in samples])
        max_us = max(int(required_number(sample, "max_us")) for sample in samples)
        mean_us = float(median([required_number(sample, "mean_us") for sample in samples]))
        stddev_us = float(median([required_number(sample, "stddev_us") for sample in samples]))
        outlier_count = median_int(
            [required_number(sample, "outlier_high_count") for sample in samples]
        )
        outlier_ratio = float(
            median([required_number(sample, "outlier_high_ratio") for sample in samples])
        )
        frame_budget = median_int(
            [required_number(sample, "frame_budget_us") for sample in samples]
        )
        frame_jank_count = median_int(
            [required_number(sample, "frame_jank_count") for sample in samples]
        )
        frame_jank_ratio = float(
            median([required_number(sample, "frame_jank_ratio") for sample in samples])
        )
        missed_count = median_int(
            [required_number(sample, "missed_present_proxy_count") for sample in samples]
        )
        missed_ratio = float(
            median([required_number(sample, "missed_present_proxy_ratio") for sample in samples])
        )
        p95_spread = max(p95_values) - min(p95_values) if len(p95_values) > 1 else 0

        warn_limit = int(os.getenv(scenario["warn_env"], str(scenario["warn_default"])))
        fail_limit = None
        fail_raw = os.getenv(scenario["fail_env"])
        if fail_raw is not None:
            fail_limit = int(fail_raw)
        elif scenario["fail_default"] is not None:
            fail_limit = int(scenario["fail_default"])

        status = f"(warn>{warn_limit}us"
        if fail_limit is not None:
            status += f", fail>{fail_limit}us"
        status += ")"
        print(
            f"[perf_guard] {key}: p50={p50}us p95={p95}us p99={p99}us "
            f"max={max_us}us mean={mean_us:.1f}us stddev={stddev_us:.1f}us "
            f"outliers={outlier_count} ({outlier_ratio * 100.0:.1f}%) "
            f"runs={len(samples)} p95_spread={p95_spread}us {status}"
        )
        print(
            f"[perf_guard]   {key} frame_quality_proxy: budget={frame_budget}us "
            f"jank={frame_jank_count} ({frame_jank_ratio * 100.0:.1f}%) "
            f"missed_present={missed_count} ({missed_ratio * 100.0:.1f}%) "
            f"(warn_jank>{warn_jank_ratio * 100.0:.1f}% "
            f"warn_missed>{warn_missed_ratio * 100.0:.1f}%)"
        )
        emit_stage_attribution(key, gui_reports, len(samples))
        emit_segment_attribution(key, gui_reports)
        emit_rebuild_attribution(key, gui_reports)

        if p95 > warn_limit:
            warned = True
            contributors.append((p95 / max(warn_limit, 1), key, p95, warn_limit))
            warn(f"{key} median p95 {p95}us exceeded warning limit {warn_limit}us")
        if fail_limit is not None and p95 > fail_limit:
            failed = True
            print(
                f"[perf_guard] ERROR: {key} median p95 {p95}us exceeded fail limit {fail_limit}us",
                file=sys.stderr,
            )
        if frame_jank_ratio > warn_jank_ratio:
            warned = True
            jank_contributors.append(
                (
                    frame_jank_ratio / max(warn_jank_ratio, 1e-9),
                    key,
                    "jank_ratio",
                    frame_jank_ratio * 100.0,
                    warn_jank_ratio * 100.0,
                )
            )
            warn(
                f"{key} median frame_jank_ratio {frame_jank_ratio * 100.0:.1f}% "
                f"exceeded warning limit {warn_jank_ratio * 100.0:.1f}%"
            )
        if fail_jank_ratio is not None and frame_jank_ratio > fail_jank_ratio:
            failed = True
            print(
                f"[perf_guard] ERROR: {key} median frame_jank_ratio "
                f"{frame_jank_ratio * 100.0:.1f}% exceeded fail limit "
                f"{fail_jank_ratio * 100.0:.1f}%",
                file=sys.stderr,
            )
        if missed_ratio > warn_missed_ratio:
            warned = True
            jank_contributors.append(
                (
                    missed_ratio / max(warn_missed_ratio, 1e-9),
                    key,
                    "missed_present_ratio",
                    missed_ratio * 100.0,
                    warn_missed_ratio * 100.0,
                )
            )
            warn(
                f"{key} median missed_present_proxy_ratio {missed_ratio * 100.0:.1f}% "
                f"exceeded warning limit {warn_missed_ratio * 100.0:.1f}%"
            )
        if fail_missed_ratio is not None and missed_ratio > fail_missed_ratio:
            failed = True
            print(
                f"[perf_guard] ERROR: {key} median missed_present_proxy_ratio "
                f"{missed_ratio * 100.0:.1f}% exceeded fail limit "
                f"{fail_missed_ratio * 100.0:.1f}%",
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
    return warned, failed


def main() -> int:
    args = parse_args()
    try:
        contract = load_json(Path(args.contract))
        gui_reports = load_gui_reports([Path(path) for path in args.reports])
        emit_projection_diagnostics(gui_reports)
        warned, failed = evaluate_scenarios(contract, gui_reports)
    except ValueError as exc:
        print(f"[perf_guard] ERROR: {exc}", file=sys.stderr)
        return 1

    if warned:
        print("[perf_guard] WARN: latency drift detected (warn-only mode)")
    else:
        print("[perf_guard] OK: all scenario p95 values within warning limits")
    return 2 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
