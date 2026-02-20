# Agent Memory

Last Updated: 2026-02-20T11:30:36Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am completing Phase 4 follow-through in
  `docs/plans/active/runtime_performance_exec_plan.md`.
- I added stage-attributed interaction benchmark reporting
  (`input`/`apply`/`pull`/`projection`) and exposed it in GUI benchmark JSON
  under `interaction_stage_attribution`.
- I updated `scripts/run_perf_guard.sh` to print per-scenario stage p95
  attribution when benchmark output includes stage fields.
- I promoted `browser_focus_commit_latency` to a default hard-fail perf-guard
  threshold (`SEMPAL_PERF_FAIL_P95_US_FOCUS_COMMIT=100000`) while keeping
  higher-variance scenarios warning-only.
- `bash scripts/ci_local.sh` is green and I am preparing commit/push for this
  milestone.

## Work Notes

- Latest pushed commits:
  - `vendor/radiant`: `cb9999b` (`perf(native_vello): intern text layout keys and atom cache`)
  - `sempal`: `f7381c03` (`perf(runtime): reduce text churn and queue lock contention`)
- Pending commit (not yet pushed): stage-attributed benchmark reporting +
  perf-guard focus-commit hard-fail promotion and docs updates.
