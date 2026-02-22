# Agent Memory

Last Updated: 2026-02-22T10:49:31Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-22 UTC)

- I attacked startup-baseline item #1 by tightening startup capture and
  calibration tooling:
  - `scripts/run_perf_guard.sh` now prebuilds `sempal` for startup captures,
    supports minimum-valid startup run requirements, and forwards startup spread
    thresholds to summary parsing,
  - `scripts/perf_startup_summary.py` now emits richer startup stats
    (median/p95/p99/max/spread), missing-reason classification, and calibrated
    threshold recommendations (`startup_first_paint_recommended`).
- I ran multi-run startup capture attempts and confirmed this environment cannot
  produce first-present samples because no Wayland compositor is available
  (`no_wayland_compositor`), so guard calibration now reports that reason
  explicitly instead of silently producing empty output.
- I updated startup profiling docs in `docs/ENV_VARS.md` and
  `docs/performance_qa.md` for min-valid-run enforcement and calibration flow.

## Work Notes

- Current focus remains runtime perf/responsiveness milestones from
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Next steps are tracked in `docs/plans/active/todo.md`.
