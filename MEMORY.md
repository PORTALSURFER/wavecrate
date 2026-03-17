# Agent Memory

Last Updated: 2026-03-17T09:45:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is a merged evidence-driven improvement audit backlog.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- `tmp/improvement_audit_plan.md` now records the merged Phase 2 execution plan for the live tree.
- Execution is in progress against the merged backlog.
- Current full-scan guardrails are degraded because `src/app/controller/library/background_jobs/analysis.rs` exceeds the file-size budget and triggers `scripts/check_quality_score_drift.ps1`.
- The merged backlog carries forward completed work for cleanup artifact refresh, waveform-segment splitting, and native text-renderer splitting, while the remaining controller/runtime items are still pending.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Execute the remaining merged backlog items in `tmp/improvement_audit_plan.md` sequentially.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned around the merged Phase 2 audit execution state.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Treat `scripts/ci_quick.ps1` as the default pre-push validation gate on Windows and `scripts/ci_local.ps1` as the broader parity baseline for validation/tooling changes once the baseline file-size guardrails are back to green.

## Work Notes

- Active audit execution record: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`


