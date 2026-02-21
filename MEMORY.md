# Agent Memory

Last Updated: 2026-02-21T09:58:56Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-21 UTC)

- I am tightening runtime responsiveness in the Phase 7 invalidation-scope lane.
- I changed radiant static-scene rebuild behavior so model refreshes stay
  dirty-mask-driven unless static rebuild is explicitly requested.
- I added focused radiant tests for static rebuild resolution behavior when the
  bridge dirty mask is empty, explicitly static-invalidated, and static-dirty.
- I optimized browser row projection in `src/app_core/native_shell.rs` by
  removing a temporary visible-window copy and iterating visible indexes
  directly with bounded pre-allocation.
- I validated with `bash scripts/ci_local.sh`; perf guard reports p95 values
  well under warning thresholds (for example `hover_latency` ~699us,
  `wheel_latency` ~742us, `volume_drag_latency` ~101us).

## Work Notes

- Current focus remains runtime perf/responsiveness milestones from
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Next steps are tracked in `docs/plans/active/todo.md`.
