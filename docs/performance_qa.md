---
layout: default
title: Performance QA
permalink: /performance_qa
description: Checklist for keeping huge sample libraries responsive in Sempal.
---

# Large-list Performance QA

## Goals
- Confirm 50k+ wav entries remain responsive in the native shell (scrolling, selection, drag/drop).
- Ensure background loading and status timings are reasonable and non-blocking.

## Setup
- Point a sample source at a folder with ~50k wav files (or duplicate a smaller corpus until the count is reached).
- Start with a clean run so cached labels and entries rebuild once.
- Enable renderer profiling only when needed:
  - Build `radiant` with `--features gui-performance`.
  - Set `SEMPAL_NATIVE_RENDER_PROFILE=1` before launch.
  - Profiling prints averages every 240 native redraw frames to stderr; disable feature for normal runs to avoid collection overhead.
- Run `bash scripts/run_perf_guard.sh` (or `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`) to execute deterministic benchmark scenarios for hover, wheel, map-pan proxy, and waveform interactions.
- `hover_latency` in perf guard reflects preview-hover behavior (focus-only row hover without commit/load side effects).
- Tune guard input size, run count, and thresholds with `SEMPAL_PERF_GUARD_*`,
  `SEMPAL_PERF_WARN_P95_US_*`, and `SEMPAL_PERF_FAIL_P95_US_*` overrides
  documented in `docs/ENV_VARS.md`.
- For stability checks, prefer a repeated-run protocol:
  - `SEMPAL_PERF_GUARD_RUNS=3`
  - keep warmup/measure counts fixed across comparisons
  - compare median p95/p99 and `p95_spread` deltas between branches
- For wheel promotion readiness, run:
  - `bash scripts/run_perf_wheel_stability.sh collect-and-evaluate`
  - review `target/perf/wheel_stability/wheel_stability_summary.json`
  - require `ready_for_fail_promotion=true` across the required window count

## Checklist
- Launch app and select the large source.
- Status should read `Loaded/Cached <count> wav files in <ms>`; note the timing.
- Scroll the triage columns quickly; verify no stutter and no duplicate-ID warnings.
- Select random rows; ensure waveform updates and status remains responsive.
- Drag a few rows onto folders or triage columns; confirm hover highlight and drop works without lag.
- Switch between sources and back; large source should reuse cached labels and load instantly.
- Trigger manual scan and confirm UI stays responsive and reload status updates with new timing.

## Metrics to capture
- Initial load time (ms) from status.
- Frame responsiveness during fast scroll (subjective) and selection latency.
- Post-scan reload time (ms).
- Perf guard scenario latencies (p50/p95/p99/max/mean/stddev), outlier counts,
  and warning/fail-threshold drift across runs.
- Perf guard stage attribution (where available) for `input`, `apply`, `pull`,
  and `projection` p95 values so drift can be localized quickly.
- Wheel-promotion readiness summary (`ready_for_fail_promotion`, reasons, and
  per-window stability metrics) from `run_perf_wheel_stability`.

## Follow-ups
- Adjust row height/window size if scroll perf regresses.
- If load time exceeds comfort, consider chunked DB reads or async batching.
