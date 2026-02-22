#!/usr/bin/env python3
"""Lock frame-quality perf-guard threshold recommendations into an env file."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from statistics import median

SCENARIOS = (
    "browser_filter_churn_latency",
    "browser_query_churn_latency",
    "browser_sort_toggle_latency",
    "hover_latency",
    "wheel_latency",
    "browser_focus_preview_latency",
    "browser_focus_commit_latency",
    "map_pan_proxy_latency",
    "waveform_interaction_latency",
    "waveform_pan_zoom_adjacent_latency",
    "volume_drag_latency",
)


@dataclass(frozen=True)
class FrameQualityThresholds:
    """Recommended frame-quality warning/fail threshold values."""

    warn_jank_ratio: float
    fail_jank_ratio: float
    warn_missed_present_ratio: float
    fail_missed_present_ratio: float
    observed_max_median_jank_ratio: float
    observed_max_median_missed_present_ratio: float


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments for frame-quality threshold lock output."""
    parser = argparse.ArgumentParser(
        description=(
            "Read sempal-bench GUI reports and write frame-quality threshold env assignments."
        )
    )
    parser.add_argument(
        "--out",
        required=True,
        help="Output env file path to write threshold assignments.",
    )
    parser.add_argument(
        "--min-runs",
        type=int,
        default=3,
        help="Minimum valid benchmark reports required before writing output.",
    )
    parser.add_argument(
        "reports",
        nargs="+",
        help="One or more sempal-bench JSON report paths.",
    )
    return parser.parse_args()


def parse_report(path: Path) -> dict[str, object]:
    """Load one benchmark report and return its `gui` section."""
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise RuntimeError(f"failed to read benchmark report {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"failed to parse benchmark report JSON {path}: {exc}") from exc
    gui = payload.get("gui")
    if not isinstance(gui, dict):
        raise RuntimeError(f"benchmark report {path} is missing a `gui` section")
    return gui


def collect_scenario_ratios(
    gui_reports: list[dict[str, object]],
    scenario_key: str,
) -> tuple[list[float], list[float]]:
    """Collect jank and missed-present proxy ratios for one scenario across runs."""
    jank_ratios: list[float] = []
    missed_present_ratios: list[float] = []
    for gui in gui_reports:
        summary = gui.get(scenario_key)
        if not isinstance(summary, dict):
            continue
        jank = summary.get("frame_jank_ratio")
        missed = summary.get("missed_present_proxy_ratio")
        if not isinstance(jank, (int, float)) or not isinstance(missed, (int, float)):
            continue
        jank_ratios.append(float(jank))
        missed_present_ratios.append(float(missed))
    return jank_ratios, missed_present_ratios


def recommend_thresholds(gui_reports: list[dict[str, object]]) -> FrameQualityThresholds:
    """Derive calibrated frame-quality threshold suggestions from benchmark reports."""
    scenario_median_jank: list[float] = []
    scenario_median_missed_present: list[float] = []
    for scenario_key in SCENARIOS:
        jank_values, missed_values = collect_scenario_ratios(gui_reports, scenario_key)
        if not jank_values or not missed_values:
            continue
        scenario_median_jank.append(float(median(jank_values)))
        scenario_median_missed_present.append(float(median(missed_values)))

    if not scenario_median_jank:
        raise RuntimeError("reports do not contain frame-quality metrics for any scenario")

    max_median_jank = max(scenario_median_jank)
    max_median_missed_present = max(scenario_median_missed_present)

    # Keep floors aligned with existing guard defaults while adapting upward when
    # observed medians are higher.
    warn_jank = max(0.10, round(max_median_jank * 2.0 + 0.02, 3))
    fail_jank = max(0.20, round(warn_jank * 1.8, 3))
    warn_missed_present = max(0.05, round(max_median_missed_present * 2.0 + 0.01, 3))
    fail_missed_present = max(0.10, round(warn_missed_present * 1.8, 3))

    return FrameQualityThresholds(
        warn_jank_ratio=warn_jank,
        fail_jank_ratio=fail_jank,
        warn_missed_present_ratio=warn_missed_present,
        fail_missed_present_ratio=fail_missed_present,
        observed_max_median_jank_ratio=max_median_jank,
        observed_max_median_missed_present_ratio=max_median_missed_present,
    )


def render_env(
    report_paths: list[Path],
    thresholds: FrameQualityThresholds,
) -> str:
    """Render env assignments for frame-quality threshold locks."""
    generated_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    report_list = ", ".join(str(path) for path in report_paths)
    lines = [
        "# Generated by scripts/perf_frame_quality_lock_thresholds.py",
        f"# Generated at (UTC): {generated_at}",
        f"# Source reports: {report_list}",
        (
            "# Observed max median frame_jank_ratio="
            f"{thresholds.observed_max_median_jank_ratio:.6f}"
        ),
        (
            "# Observed max median missed_present_proxy_ratio="
            f"{thresholds.observed_max_median_missed_present_ratio:.6f}"
        ),
        f"SEMPAL_PERF_WARN_FRAME_JANK_RATIO={thresholds.warn_jank_ratio:.3f}",
        f"SEMPAL_PERF_FAIL_FRAME_JANK_RATIO={thresholds.fail_jank_ratio:.3f}",
        (
            "SEMPAL_PERF_WARN_MISSED_PRESENT_PROXY_RATIO="
            f"{thresholds.warn_missed_present_ratio:.3f}"
        ),
        (
            "SEMPAL_PERF_FAIL_MISSED_PRESENT_PROXY_RATIO="
            f"{thresholds.fail_missed_present_ratio:.3f}"
        ),
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    """Entry point for frame-quality threshold lock helper."""
    args = parse_args()
    report_paths = [Path(raw_path) for raw_path in args.reports]
    if len(report_paths) < args.min_runs:
        print(
            "[perf_guard] ERROR: insufficient benchmark reports supplied: "
            f"{len(report_paths)} < {args.min_runs}",
            file=sys.stderr,
        )
        return 2

    try:
        gui_reports = [parse_report(path) for path in report_paths]
        thresholds = recommend_thresholds(gui_reports)
        rendered = render_env(report_paths, thresholds)
        out_path = Path(args.out)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(rendered, encoding="utf-8")
    except RuntimeError as exc:
        print(f"[perf_guard] ERROR: {exc}", file=sys.stderr)
        return 2

    print(f"[perf_guard] frame-quality threshold lock file written: {args.out}")
    print(
        "[perf_guard] frame-quality threshold lock values: "
        f"warn_jank={thresholds.warn_jank_ratio:.3f} "
        f"fail_jank={thresholds.fail_jank_ratio:.3f} "
        f"warn_missed={thresholds.warn_missed_present_ratio:.3f} "
        f"fail_missed={thresholds.fail_missed_present_ratio:.3f}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
