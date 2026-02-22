#!/usr/bin/env python3
"""Summarize native startup timing profile logs for perf guard workflows."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from math import ceil
from pathlib import Path
from statistics import median

LOG_PREFIX = "[native-vello-startup]"
EXPECTED_METRICS = (
    "window_create_ms",
    "surface_ready_ms",
    "renderer_ready_ms",
    "first_scene_ready_ms",
    "first_present_ms",
    "deferred_model_refresh_ms",
    "deferred_model_refresh_total_ms",
)
METRIC_PAIR_RE = re.compile(r"([a-z_]+_ms)=([0-9]+(?:\.[0-9]+)?)")


class ParsedLog:
    """Parsed startup log result."""

    def __init__(self, path: Path, metrics: dict[str, float] | None, reason: str | None) -> None:
        self.path = path
        self.metrics = metrics
        self.reason = reason



def parse_args() -> argparse.Namespace:
    """Parse CLI arguments for startup profile summarization."""
    parser = argparse.ArgumentParser(
        description=(
            "Parse native startup timing logs emitted by "
            "SEMPAL_NATIVE_STARTUP_PROFILE and print a guard summary."
        )
    )
    parser.add_argument(
        "log_paths",
        nargs="+",
        help="One or more startup profiling log files to parse.",
    )
    parser.add_argument(
        "--output",
        default=None,
        help="Optional JSON output path for the aggregated summary.",
    )
    parser.add_argument(
        "--warn-first-present-ms",
        type=float,
        default=800.0,
        help="Warning threshold for median first-present startup latency in ms.",
    )
    parser.add_argument(
        "--fail-first-present-ms",
        type=float,
        default=None,
        help="Optional fail threshold for median first-present startup latency in ms.",
    )
    parser.add_argument(
        "--warn-first-present-spread-ms",
        type=float,
        default=None,
        help="Optional warning threshold for first-present spread across runs in ms.",
    )
    parser.add_argument(
        "--fail-first-present-spread-ms",
        type=float,
        default=None,
        help="Optional fail threshold for first-present spread across runs in ms.",
    )
    parser.add_argument(
        "--min-valid-runs",
        type=int,
        default=1,
        help="Minimum number of logs that must contain startup profile lines.",
    )
    parser.add_argument(
        "--require-min-valid-runs",
        action="store_true",
        help="Fail when valid startup profile runs are below --min-valid-runs.",
    )
    return parser.parse_args()



def parse_startup_line(line: str) -> dict[str, float] | None:
    """Parse one startup profile line into metric values."""
    if LOG_PREFIX not in line:
        return None
    values: dict[str, float] = {}
    for key, value in METRIC_PAIR_RE.findall(line):
        values[key] = float(value)
    if not all(metric in values for metric in EXPECTED_METRICS):
        return None
    return values



def classify_missing_reason(lines: list[str]) -> str:
    """Classify why a startup profile line is missing from one log."""
    text = "\n".join(lines)
    if "Could not find wayland compositor" in text:
        return "no_wayland_compositor"
    if "no X server" in text or "DISPLAY" in text and "error" in text.lower():
        return "display_backend_error"
    if "Running `target/debug/sempal`" not in text:
        return "build_or_launch_timeout"
    if "runtime returned error" in text:
        return "runtime_error_before_first_present"
    return "missing_profile_line"



def parse_log(path: Path) -> ParsedLog:
    """Return the latest valid startup profile line from one log file."""
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError as exc:
        print(
            f"[perf_guard] WARN: failed to read startup profile log {path}: {exc}",
            file=sys.stderr,
        )
        return ParsedLog(path, None, "read_error")
    latest: dict[str, float] | None = None
    for line in lines:
        parsed = parse_startup_line(line)
        if parsed is not None:
            latest = parsed
    if latest is None:
        reason = classify_missing_reason(lines)
        print(
            f"[perf_guard] WARN: startup profile line missing in {path} ({reason})",
            file=sys.stderr,
        )
        return ParsedLog(path, None, reason)
    return ParsedLog(path, latest, None)



def percentile(sorted_values: list[float], quantile: float) -> float:
    """Return percentile by nearest-rank style index for a sorted sample."""
    if not sorted_values:
        return 0.0
    index = int(round((len(sorted_values) - 1) * quantile))
    index = min(max(index, 0), len(sorted_values) - 1)
    return float(sorted_values[index])



def summarize(records: list[dict[str, float]]) -> dict[str, dict[str, float]]:
    """Compute summary stats per startup metric across records."""
    summary: dict[str, dict[str, float]] = {}
    for metric in EXPECTED_METRICS:
        values = sorted(float(record[metric]) for record in records)
        metric_min = values[0]
        metric_max = values[-1]
        summary[metric] = {
            "min": metric_min,
            "median": float(median(values)),
            "p95": percentile(values, 0.95),
            "p99": percentile(values, 0.99),
            "max": metric_max,
            "spread": metric_max - metric_min,
        }
    return summary



def recommend_thresholds(metric_summary: dict[str, dict[str, float]]) -> dict[str, int]:
    """Derive calibrated threshold suggestions from observed first-present data."""
    first_present = metric_summary["first_present_ms"]
    spread = first_present["spread"]
    warn_first_present = ceil(
        max(
            first_present["p95"] * 1.10,
            first_present["median"] + max(50.0, spread * 1.5),
        )
    )
    fail_first_present = ceil(
        max(
            first_present["p99"] * 1.20,
            float(warn_first_present) * 1.6,
            first_present["max"] + 75.0,
        )
    )
    warn_spread = ceil(max(100.0, spread * 1.5))
    fail_spread = ceil(max(float(warn_spread) * 1.8, spread * 2.5))
    return {
        "warn_first_present_ms": warn_first_present,
        "fail_first_present_ms": fail_first_present,
        "warn_first_present_spread_ms": warn_spread,
        "fail_first_present_spread_ms": fail_spread,
    }



def write_summary(
    output_path: Path | None,
    metric_summary: dict[str, dict[str, float]] | None,
    total_runs: int,
    valid_runs: int,
    missing_reasons: Counter[str],
    recommendations: dict[str, int] | None,
) -> None:
    """Persist startup summary JSON when an output path is provided."""
    if output_path is None:
        return
    payload = {
        "total_runs": total_runs,
        "runs_with_profile": valid_runs,
        "runs_missing_profile": total_runs - valid_runs,
        "missing_reason_counts": dict(sorted(missing_reasons.items())),
        "metric_names": list(EXPECTED_METRICS),
        "metrics_ms": metric_summary,
        "recommended_thresholds_ms": recommendations,
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")



def print_summary_line(
    metric_summary: dict[str, dict[str, float]],
    total_runs: int,
    valid_runs: int,
    args: argparse.Namespace,
) -> None:
    """Print one human-readable startup summary line for perf guard logs."""
    first_present = metric_summary["first_present_ms"]
    status = f"(warn>{args.warn_first_present_ms:.3f}ms"
    if args.fail_first_present_ms is not None:
        status += f", fail>{args.fail_first_present_ms:.3f}ms"
    if args.warn_first_present_spread_ms is not None:
        status += f", warn_spread>{args.warn_first_present_spread_ms:.3f}ms"
    if args.fail_first_present_spread_ms is not None:
        status += f", fail_spread>{args.fail_first_present_spread_ms:.3f}ms"
    status += ")"
    print(
        "[perf_guard] startup_first_paint: "
        f"window_create_ms={metric_summary['window_create_ms']['median']:.3f} "
        f"surface_ready_ms={metric_summary['surface_ready_ms']['median']:.3f} "
        f"renderer_ready_ms={metric_summary['renderer_ready_ms']['median']:.3f} "
        f"first_scene_ready_ms={metric_summary['first_scene_ready_ms']['median']:.3f} "
        f"first_present_ms={first_present['median']:.3f} "
        f"first_present_p95_ms={first_present['p95']:.3f} "
        f"first_present_p99_ms={first_present['p99']:.3f} "
        f"first_present_spread_ms={first_present['spread']:.3f} "
        f"deferred_model_refresh_ms={metric_summary['deferred_model_refresh_ms']['median']:.3f} "
        f"deferred_model_refresh_total_ms={metric_summary['deferred_model_refresh_total_ms']['median']:.3f} "
        f"runs={valid_runs}/{total_runs} "
        f"{status}"
    )



def main() -> int:
    """Parse startup logs, print perf summary, and enforce thresholds."""
    args = parse_args()
    parsed_logs = [parse_log(Path(raw_path)) for raw_path in args.log_paths]
    records = [parsed.metrics for parsed in parsed_logs if parsed.metrics is not None]
    missing_reasons = Counter(
        parsed.reason for parsed in parsed_logs if parsed.reason is not None
    )

    total_runs = len(parsed_logs)
    valid_runs = len(records)
    metric_summary: dict[str, dict[str, float]] | None = None
    recommendations: dict[str, int] | None = None

    if records:
        metric_summary = summarize(records)
        recommendations = recommend_thresholds(metric_summary)
        print_summary_line(metric_summary, total_runs, valid_runs, args)
        print(
            "[perf_guard] startup_first_paint_recommended: "
            f"warn_first_present_ms={recommendations['warn_first_present_ms']} "
            f"fail_first_present_ms={recommendations['fail_first_present_ms']} "
            f"warn_spread_ms={recommendations['warn_first_present_spread_ms']} "
            f"fail_spread_ms={recommendations['fail_first_present_spread_ms']}"
        )
    else:
        print(
            "[perf_guard] WARN: native startup profile capture produced no timing records",
            file=sys.stderr,
        )

    if missing_reasons:
        reason_summary = " ".join(
            f"{reason}={count}" for reason, count in sorted(missing_reasons.items())
        )
        print(
            f"[perf_guard] startup_profile_missing_reasons: {reason_summary}",
            file=sys.stderr,
        )

    if args.require_min_valid_runs and valid_runs < args.min_valid_runs:
        print(
            "[perf_guard] ERROR: startup profile valid run count "
            f"{valid_runs} is below required minimum {args.min_valid_runs}",
            file=sys.stderr,
        )
        write_summary(
            Path(args.output) if args.output else None,
            metric_summary,
            total_runs,
            valid_runs,
            missing_reasons,
            recommendations,
        )
        return 2

    status = 0
    if metric_summary is not None:
        first_present = metric_summary["first_present_ms"]
        if first_present["median"] > args.warn_first_present_ms:
            print(
                "[perf_guard] WARN: startup_first_paint median first_present_ms "
                f"{first_present['median']:.3f} exceeded warning limit "
                f"{args.warn_first_present_ms:.3f}",
                file=sys.stderr,
            )
        if (
            args.fail_first_present_ms is not None
            and first_present["median"] > args.fail_first_present_ms
        ):
            print(
                "[perf_guard] ERROR: startup_first_paint median first_present_ms "
                f"{first_present['median']:.3f} exceeded fail limit "
                f"{args.fail_first_present_ms:.3f}",
                file=sys.stderr,
            )
            status = 2
        if (
            args.warn_first_present_spread_ms is not None
            and first_present["spread"] > args.warn_first_present_spread_ms
        ):
            print(
                "[perf_guard] WARN: startup_first_paint first_present_spread_ms "
                f"{first_present['spread']:.3f} exceeded warning limit "
                f"{args.warn_first_present_spread_ms:.3f}",
                file=sys.stderr,
            )
        if (
            args.fail_first_present_spread_ms is not None
            and first_present["spread"] > args.fail_first_present_spread_ms
        ):
            print(
                "[perf_guard] ERROR: startup_first_paint first_present_spread_ms "
                f"{first_present['spread']:.3f} exceeded fail limit "
                f"{args.fail_first_present_spread_ms:.3f}",
                file=sys.stderr,
            )
            status = 2

    write_summary(
        Path(args.output) if args.output else None,
        metric_summary,
        total_runs,
        valid_runs,
        missing_reasons,
        recommendations,
    )
    return status


if __name__ == "__main__":
    raise SystemExit(main())
