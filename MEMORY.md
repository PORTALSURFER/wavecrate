# Agent Memory

Last Updated: 2026-02-22T10:38:26Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-22 UTC)

- I completed the next perf workflow batch by:
  - adding perf-guard startup-profile ingestion in `scripts/run_perf_guard.sh`
    (optional `SEMPAL_PERF_GUARD_STARTUP_PROFILE=1` capture path plus startup
    summary emission),
  - adding `scripts/perf_startup_summary.py` to parse
    `SEMPAL_NATIVE_STARTUP_PROFILE` logs and emit aggregated startup metrics +
    warning/fail threshold checks,
  - updating runtime bridge handling so `MoveBrowserFocus` is applied
    immediately in `app_core` instead of waiting for queued flush.
- I updated docs for the new startup profiling/perf-guard knobs in
  `docs/ENV_VARS.md` and `docs/performance_qa.md`.
- I validated with `bash scripts/ci_local.sh`; all checks passed (perf guard
  stayed non-failing, with one warning-only hover latency drift run).

## Work Notes

- Current focus remains runtime perf/responsiveness milestones from
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Next steps are tracked in `docs/plans/active/todo.md`.
