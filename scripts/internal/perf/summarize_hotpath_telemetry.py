#!/usr/bin/env python3
"""Summarize Wavecrate hot-path telemetry logs for interactive playback smoke runs."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from statistics import mean, median
from typing import Any

FIELD_RE = re.compile(
    r"([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:\"((?:\\.|[^\"\\])*)\"|([^\"\s]+))"
)

TELEMETRY_MARKERS = (
    "perf::audio_start",
    "perf::starmap_drag",
    "perf::hotpath",
    "browser.sample_load.preview_audition",
    "fast_audition.",
    "preview_audition.",
    "starmap_audition",
    "ui.frame.dispatch_profile",
)

GROUP_FIELDS = (
    "outcome",
    "origin",
    "probe",
    "stage",
    "counter",
    "reason",
    "source_kind",
    "display_mode",
    "active",
)

TIMING_FIELDS = (
    "elapsed_ms",
    "worker_elapsed_ms",
    "commit_elapsed_ms",
    "avg_focus_ms",
    "max_focus_ms",
    "avg_widget_hit_test_ms",
    "max_widget_hit_test_ms",
    "avg_widget_paint_build_ms",
    "max_widget_paint_build_ms",
    "avg_ready_source_ms",
    "max_ready_source_ms",
    "avg_runtime_start_ms",
    "max_runtime_start_ms",
    "avg_start_total_ms",
    "max_start_total_ms",
)

SUM_FIELDS = (
    "scheduled",
    "attempted",
    "decoded",
    "errors",
    "inspected",
    "candidates",
    "eligible",
    "hit_count",
    "queue_len",
    "hits_queued",
    "hits_started",
    "ready_started",
    "ready_pending",
    "ready_unavailable",
    "validation_queued",
    "runtime_started",
    "runtime_failed",
    "runtime_cancelled",
    "runtime_stale",
)

METRIC_FIELDS = (
    "scheduled",
    "attempted",
    "decoded",
    "errors",
    "inspected",
    "candidates",
    "eligible",
    "starmap_cells",
    "starmap_visited_cells",
    "starmap_remaining_budget",
    "list_remaining_budget",
    "hit_count",
    "queue_len",
    "hits_queued",
    "hits_started",
    "ready_unavailable",
    "max_queue_len",
)

UI_FRAME_BUDGET_MS = 16.7
UI_FRAME_PHASE_BUDGET_MS = 8.0


@dataclass
class ParsedEvent:
    """One parsed telemetry line."""

    path: str
    line_number: int
    event: str
    fields: dict[str, str]
    line: str


@dataclass
class EventGroup:
    """Aggregated telemetry records for one canonical event."""

    records: list[ParsedEvent] = field(default_factory=list)
    counts: dict[str, Counter[str]] = field(
        default_factory=lambda: defaultdict(Counter)
    )
    timings: dict[str, list[float]] = field(default_factory=lambda: defaultdict(list))
    metrics: dict[str, list[int]] = field(default_factory=lambda: defaultdict(list))
    totals: Counter[str] = field(default_factory=Counter)

    def add(self, record: ParsedEvent) -> None:
        self.records.append(record)
        for key in GROUP_FIELDS:
            value = record.fields.get(key)
            if value:
                self.counts[key][value] += 1
        for key in timing_field_names(record.fields):
            value = parse_float(record.fields.get(key))
            if value is not None:
                self.timings[key].append(value)
        for key in METRIC_FIELDS:
            value = parse_int(record.fields.get(key))
            if value is not None:
                self.metrics[key].append(value)
        for key in SUM_FIELDS:
            value = parse_int(record.fields.get(key))
            if value is not None:
                self.totals[key] += value


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Summarize WAVECRATE_HOTPATH_TELEMETRY logs so dense starmap "
            "drag/list/keyboard playback smoke runs show where latency or "
            "misses accumulate."
        )
    )
    parser.add_argument(
        "logs",
        nargs="+",
        help="One or more Wavecrate log files to parse, or '-' for stdin.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit the aggregate summary as JSON instead of text.",
    )
    parser.add_argument(
        "--top",
        type=int,
        default=6,
        help="Maximum number of grouped values to print per field.",
    )
    return parser.parse_args()


def parse_fields(line: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for match in FIELD_RE.finditer(line):
        key = match.group(1)
        quoted = match.group(2)
        bare = match.group(3)
        fields[key] = unescape_quoted(quoted) if quoted is not None else bare
    return fields


def unescape_quoted(value: str) -> str:
    return value.replace(r"\"", '"').replace(r"\\", "\\")


def parse_float(value: str | None) -> float | None:
    if value is None:
        return None
    try:
        return float(value)
    except ValueError:
        return None


def parse_int(value: str | None) -> int | None:
    if value is None:
        return None
    try:
        return int(float(value))
    except ValueError:
        return None


def timing_field_names(fields: dict[str, str]) -> list[str]:
    names = {
        key
        for key in fields
        if key in TIMING_FIELDS or key.endswith("_ms")
    }
    return sorted(names)


def line_is_relevant(line: str) -> bool:
    return any(marker in line for marker in TELEMETRY_MARKERS)


def canonical_event(fields: dict[str, str], line: str) -> str:
    event = fields.get("event", "")
    if event.endswith("preview_audition.warm_plan"):
        return "preview_audition.warm_plan"
    if event.endswith("preview_audition.warm_phase_profile"):
        return "preview_audition.warm_phase_profile"
    if event.endswith("preview_audition.warm_finished"):
        return "preview_audition.warm_finished"
    if event == "fast_audition.decision":
        return event
    if event == "fast_audition.cancel_replaced_load":
        return event
    if event == "ui.frame.dispatch_profile":
        return event
    if fields.get("module") == "starmap_audition":
        if "events_total" in fields:
            return "starmap_audition.snapshot"
        return "starmap_audition.event"
    if "Preview audition warm finished" in line:
        return "preview_audition.warm_finished"
    if "Preview audition warm plan" in line:
        return "preview_audition.warm_plan"
    if "Preview audition warm phase profile" in line:
        return "preview_audition.warm_phase_profile"
    if "Slow preview audition warm phase" in line:
        return "preview_audition.warm_phase_profile"
    return event or "unknown"


def parse_lines(path: str, lines: list[str]) -> list[ParsedEvent]:
    records: list[ParsedEvent] = []
    for line_number, line in enumerate(lines, start=1):
        if not line_is_relevant(line):
            continue
        fields = parse_fields(line)
        event = canonical_event(fields, line)
        if event == "unknown":
            continue
        records.append(
            ParsedEvent(
                path=path,
                line_number=line_number,
                event=event,
                fields=fields,
                line=line.rstrip("\n"),
            )
        )
    return records


def read_log(path: str) -> list[str]:
    if path == "-":
        return sys.stdin.read().splitlines()
    return Path(path).read_text(encoding="utf-8", errors="replace").splitlines()


def percentile(values: list[float], quantile: float) -> float:
    if not values:
        return 0.0
    sorted_values = sorted(values)
    index = round((len(sorted_values) - 1) * quantile)
    return sorted_values[int(index)]


def percentile_int(values: list[int], quantile: float) -> int:
    if not values:
        return 0
    sorted_values = sorted(values)
    index = round((len(sorted_values) - 1) * quantile)
    return sorted_values[int(index)]


def summarize(records: list[ParsedEvent]) -> dict[str, Any]:
    groups: dict[str, EventGroup] = defaultdict(EventGroup)
    for record in records:
        groups[record.event].add(record)

    event_payloads: dict[str, Any] = {}
    for event, group in sorted(groups.items()):
        timings: dict[str, Any] = {}
        for key, values in sorted(group.timings.items()):
            timings[key] = {
                "count": len(values),
                "avg": mean(values),
                "p50": median(values),
                "p95": percentile(values, 0.95),
                "max": max(values),
            }
        metrics: dict[str, Any] = {}
        for key, values in sorted(group.metrics.items()):
            metrics[key] = {
                "count": len(values),
                "avg": mean(values),
                "p50": median(values),
                "p95": percentile_int(values, 0.95),
                "max": max(values),
            }
        event_payloads[event] = {
            "count": len(group.records),
            "counts": {
                key: dict(counter.most_common())
                for key, counter in sorted(group.counts.items())
            },
            "timings": timings,
            "metrics": metrics,
            "totals": dict(group.totals),
        }

    return {
        "records": len(records),
        "events": event_payloads,
        "warnings": diagnostics(groups),
    }


def diagnostics(groups: dict[str, EventGroup]) -> list[str]:
    warnings: list[str] = []
    if "starmap_audition.event" not in groups and "starmap_audition.snapshot" not in groups:
        warnings.append(
            "no starmap audition events found; normal logs only include slow starmap "
            "hotpath warnings, so run a dense drag repro with WAVECRATE_HOTPATH_TELEMETRY=1 "
            "and RUST_LOG including perf::starmap_drag for full queue/hit/playback coverage"
        )
    if "fast_audition.decision" not in groups:
        warnings.append(
            "no fast audition decisions found; ensure RUST_LOG includes perf::audio_start "
            "during drag/list/keyboard playback"
        )

    snapshot = latest_snapshot(groups.get("starmap_audition.snapshot"))
    if snapshot:
        hits_queued = parse_int(snapshot.fields.get("hits_queued")) or 0
        hits_started = parse_int(snapshot.fields.get("hits_started")) or 0
        max_queue_len = parse_int(snapshot.fields.get("max_queue_len")) or 0
        ready_unavailable = parse_int(snapshot.fields.get("ready_unavailable")) or 0
        runtime_cancelled = parse_int(snapshot.fields.get("runtime_cancelled")) or 0
        runtime_stale = parse_int(snapshot.fields.get("runtime_stale")) or 0
        if hits_queued and hits_started < hits_queued:
            warnings.append(
                f"starmap snapshot has fewer starts than queued hits "
                f"({hits_started}/{hits_queued})"
            )
        if max_queue_len > 1:
            warnings.append(f"starmap max_queue_len reached {max_queue_len}")
        if ready_unavailable:
            warnings.append(f"starmap ready_unavailable={ready_unavailable}")
        if runtime_cancelled or runtime_stale:
            warnings.append(
                f"starmap runtime cancellations/stale completions: "
                f"cancelled={runtime_cancelled} stale={runtime_stale}"
            )

    warm_finished = groups.get("preview_audition.warm_finished")
    if warm_finished and warm_finished.totals.get("errors", 0) > 0:
        warnings.append(
            f"preview warm reported {warm_finished.totals['errors']} decode errors"
        )
    warm_phase = groups.get("preview_audition.warm_phase_profile")
    if warm_phase:
        warnings.extend(warm_phase_diagnostics(warm_phase))
    frame_dispatch = groups.get("ui.frame.dispatch_profile")
    if frame_dispatch:
        warnings.extend(frame_dispatch_diagnostics(frame_dispatch))
    return warnings


def warm_phase_diagnostics(group: EventGroup) -> list[str]:
    warnings: list[str] = []
    total_values = group.timings.get("total_elapsed_ms", [])
    if total_values:
        total_p95 = percentile(total_values, 0.95)
        total_max = max(total_values)
        if total_max > UI_FRAME_PHASE_BUDGET_MS:
            warnings.append(
                f"preview warm phase exceeded {UI_FRAME_PHASE_BUDGET_MS:.1f}ms budget: "
                f"p95={total_p95:.1f}ms max={total_max:.1f}ms"
            )

    slow_parts: list[tuple[float, str, float]] = []
    for key in ("plan_elapsed_ms", "reservation_elapsed_ms", "task_schedule_elapsed_ms"):
        values = group.timings.get(key, [])
        if not values:
            continue
        phase_max = max(values)
        if phase_max > UI_FRAME_PHASE_BUDGET_MS:
            slow_parts.append((phase_max, key, percentile(values, 0.95)))
    if slow_parts:
        formatted = ", ".join(
            f"{key} p95={phase_p95:.1f}ms max={phase_max:.1f}ms"
            for phase_max, key, phase_p95 in sorted(slow_parts, reverse=True)
        )
        warnings.append(f"slow preview warm subphases: {formatted}")
    return warnings


def frame_dispatch_diagnostics(group: EventGroup) -> list[str]:
    warnings: list[str] = []
    elapsed_values = group.timings.get("elapsed_ms", [])
    if elapsed_values:
        elapsed_p95 = percentile(elapsed_values, 0.95)
        elapsed_max = max(elapsed_values)
        if elapsed_max > UI_FRAME_BUDGET_MS:
            warnings.append(
                f"ui frame dispatch exceeded {UI_FRAME_BUDGET_MS:.1f}ms budget: "
                f"p95={elapsed_p95:.1f}ms max={elapsed_max:.1f}ms"
            )

    slow_phases: list[tuple[float, str, float]] = []
    for key, values in group.timings.items():
        if key == "elapsed_ms" or not values:
            continue
        phase_max = max(values)
        if phase_max > UI_FRAME_PHASE_BUDGET_MS:
            slow_phases.append((phase_max, key, percentile(values, 0.95)))
    if slow_phases:
        formatted = ", ".join(
            f"{key} p95={phase_p95:.1f}ms max={phase_max:.1f}ms"
            for phase_max, key, phase_p95 in sorted(slow_phases, reverse=True)[:3]
        )
        warnings.append(f"slow ui frame phases: {formatted}")
    return warnings


def latest_snapshot(group: EventGroup | None) -> ParsedEvent | None:
    if group is None or not group.records:
        return None
    return group.records[-1]


def emit_text(summary: dict[str, Any], log_paths: list[str], top: int) -> None:
    print("Wavecrate hot-path telemetry summary")
    print("Logs:")
    for path in log_paths:
        print(f"  {path}")
    print(f"Telemetry records: {summary['records']}")
    print()

    events: dict[str, Any] = summary["events"]
    if not events:
        print("No hot-path telemetry events found.")
    for event, payload in events.items():
        print(f"{event}: {payload['count']}")
        emit_group_counts(payload["counts"], top)
        emit_timings(payload["timings"], top)
        emit_metrics(payload["metrics"], top)
        emit_totals(payload["totals"])
        print()

    warnings = summary["warnings"]
    if warnings:
        print("Diagnostics:")
        for warning in warnings:
            print(f"  - {warning}")


def emit_group_counts(counts: dict[str, dict[str, int]], top: int) -> None:
    for key, values in counts.items():
        if not values:
            continue
        pairs = sorted(values.items(), key=lambda item: (-item[1], item[0]))[:top]
        formatted = ", ".join(f"{value}={count}" for value, count in pairs)
        print(f"  {key}: {formatted}")


def emit_timings(timings: dict[str, dict[str, float]], top: int) -> None:
    visible = [
        (key, stats)
        for key, stats in timings.items()
        if key == "elapsed_ms" or stats["max"] > 0.0
    ]
    visible.sort(
        key=lambda item: (
            0 if item[0] == "elapsed_ms" else 1,
            -item[1]["max"],
            item[0],
        )
    )
    for key, stats in visible[:top]:
        print(
            f"  {key}: count={int(stats['count'])} "
            f"avg={stats['avg']:.3f}ms p50={stats['p50']:.3f}ms "
            f"p95={stats['p95']:.3f}ms max={stats['max']:.3f}ms"
        )


def emit_metrics(metrics: dict[str, dict[str, float]], top: int) -> None:
    visible = [
        (key, stats)
        for key, stats in metrics.items()
        if stats["max"] > 0
    ]
    visible.sort(key=lambda item: (-item[1]["max"], item[0]))
    for key, stats in visible[:top]:
        print(
            f"  {key}: count={int(stats['count'])} "
            f"avg={stats['avg']:.1f} p50={stats['p50']:.0f} "
            f"p95={stats['p95']:.0f} max={stats['max']:.0f}"
        )


def emit_totals(totals: dict[str, int]) -> None:
    interesting = {
        key: value
        for key, value in totals.items()
        if value != 0 and key not in {"hit_count", "queue_len"}
    }
    if not interesting:
        return
    formatted = ", ".join(
        f"{key}={value}" for key, value in sorted(interesting.items())
    )
    print(f"  totals: {formatted}")


def main() -> int:
    args = parse_args()
    records: list[ParsedEvent] = []
    for path in args.logs:
        try:
            records.extend(parse_lines(path, read_log(path)))
        except OSError as exc:
            print(f"[hotpath_summary] failed to read {path}: {exc}", file=sys.stderr)
            return 2

    summary = summarize(records)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        emit_text(summary, args.logs, args.top)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
