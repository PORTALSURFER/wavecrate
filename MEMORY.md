# Agent Memory

Last Updated: 2026-02-20T10:58:38Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am implementing the remaining runtime performance milestones from
  `docs/plans/active/runtime_performance_exec_plan.md`.
- I added benchmark-side latency volatility metrics (`p99`, `stddev`,
  interquartile range, and high-outlier counts) in `src/bin/bench/stats.rs`.
- I upgraded `scripts/run_perf_guard.sh` to support multi-run aggregation,
  p95-spread reporting, top warning-contributor ranking, and one promoted
  hard-fail threshold (`map_pan_proxy_latency` p95 default fail > 4000us).
- I updated performance docs/env-var docs to capture the new run protocol and
  threshold controls.
- `bash scripts/ci_local.sh` is green with the updated perf guard output and I
  am preparing commit/push for this stabilization milestone.

## Work Notes

- Latest pushed commits:
  - `vendor/radiant`: `cb9999b` (`perf(native_vello): intern text layout keys and atom cache`)
  - `sempal`: `f7381c03` (`perf(runtime): reduce text churn and queue lock contention`)
- Pending commit (not yet pushed): perf-guard stability milestone across
  `scripts/run_perf_guard.sh`, `src/bin/bench/stats.rs`, and related docs.
