# Agent Memory

Last Updated: 2026-02-22T09:18:47Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-22 UTC)

- I completed the browser-projection allocation-churn milestone by:
  - replacing tuple-based retained browser-row cache entries with a typed cache
    entry that stores a precomputed selected-path lookup hash,
  - reworking browser row projection to reuse existing row-model slots and
    mutate `String` buffers in place instead of rebuilding rows every frame.
- I kept migration boundary rules intact by routing the new cache-entry type
  through `app_core/app_api.rs`.
- I validated with `bash scripts/ci_local.sh`; all checks passed and perf guard
  stayed within thresholds.

## Work Notes

- Current focus remains runtime perf/responsiveness milestones from
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Next steps are tracked in `docs/plans/active/todo.md`.
