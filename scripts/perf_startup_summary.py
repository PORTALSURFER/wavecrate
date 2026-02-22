#!/usr/bin/env python3
"""Summarize native startup timing profile logs for perf guard workflows."""

from __future__ import annotations

import argparse
import json
import re
import sys
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


def parse_log(path: Path) -> dict[str, float] | None:
    """Return the latest valid startup profile line from one log file."""
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError as exc:
        print(
            f"[perf_guard] WARN: failed to read startup profile log {path}: {exc}",
            file=sys.stderr,
        )
        return None
    latest: dict[str, float] | None = None
    for line in lines:
        parsed = parse_startup_line(line)
        if parsed is not None:
            latest = parsed
    if latest is None:
        print(
            f"[perf_guard] WARN: startup profile line missing in {path}",
            file=sys.stderr,
        )
    return latest


def summarize(records: list[dict[str, float]]) -> dict[str, float]:
    """Compute median metric values across startup profile records."""
    summary: dict[str, float] = {}
    for metric in EXPECTED_METRICS:
        summary[metric] = float(median(record[metric] for record in records))
    return summary


def write_summary(
    output_path: Path | None,
    summary: dict[str, float] | None,
    records: list[dict[str, float]],
) -> None:
    """Persist startup summary JSON when an output path is provided."""
    if output_path is None:
        return
    payload = {
        "runs_with_profile": len(records),
        "metrics_median_ms": summary,
        "metric_names": list(EXPECTED_METRICS),
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")


def main() -> int:
    """Parse startup logs, print perf summary, and enforce thresholds."""
    args = parse_args()
    records: list[dict[str, float]] = []
    for raw_path in args.log_paths:
        path = Path(raw_path)
        record = parse_log(path)
        if record is not None:
            records.append(record)

    if not records:
        write_summary(Path(args.output) if args.output else None, None, records)
        print(
            "[perf_guard] WARN: native startup profile capture produced no timing records",
            file=sys.stderr,
        )
        return 0

    summary = summarize(records)
    first_present_spread = (
        max(record["first_present_ms"] for record in records)
        - min(record["first_present_ms"] for record in records)
        if len(records) > 1
        else 0.0
    )
    status = f"(warn>{args.warn_first_present_ms:.3f}ms"
    if args.fail_first_present_ms is not None:
        status += f", fail>{args.fail_first_present_ms:.3f}ms"
    status += ")"
    print(
        "[perf_guard] startup_first_paint: "
        f"window_create_ms={summary['window_create_ms']:.3f} "
        f"surface_ready_ms={summary['surface_ready_ms']:.3f} "
        f"renderer_ready_ms={summary['renderer_ready_ms']:.3f} "
        f"first_scene_ready_ms={summary['first_scene_ready_ms']:.3f} "
        f"first_present_ms={summary['first_present_ms']:.3f} "
        f"deferred_model_refresh_ms={summary['deferred_model_refresh_ms']:.3f} "
        f"deferred_model_refresh_total_ms={summary['deferred_model_refresh_total_ms']:.3f} "
        f"runs={len(records)} "
        f"first_present_spread_ms={first_present_spread:.3f} "
        f"{status}"
    )

    write_summary(Path(args.output) if args.output else None, summary, records)

    if summary["first_present_ms"] > args.warn_first_present_ms:
        print(
            "[perf_guard] WARN: startup_first_paint median first_present_ms "
            f"{summary['first_present_ms']:.3f} exceeded warning limit "
            f"{args.warn_first_present_ms:.3f}",
            file=sys.stderr,
        )
    if (
        args.fail_first_present_ms is not None
        and summary["first_present_ms"] > args.fail_first_present_ms
    ):
        print(
            "[perf_guard] ERROR: startup_first_paint median first_present_ms "
            f"{summary['first_present_ms']:.3f} exceeded fail limit "
            f"{args.fail_first_present_ms:.3f}",
            file=sys.stderr,
        )
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
