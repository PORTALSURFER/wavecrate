# Agent Memory

Last Updated: 2026-03-31T11:09:27Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I am executing Phase 2 of the refreshed evidence-driven improvement audit for the current live tree.
- `tmp/improvement_audit_plan.md` is the current source of truth and execution record for this lane.
- Item 1 is complete: the Windows file-size budget now counts physical lines, the script guardrail fixture covers blank lines, and the quality-score drift checks now evaluate the full-scan budget state.
- The truthful full-scan budget is currently red and reports `25` non-allowlisted over-budget Rust files on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1 -SkipCi` is green on the current tree.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md`.
- Item 2 is complete: loaded-sample similarity query construction now goes through one shared helper module, and a parity test compares the sync and background builders against the same seeded source snapshot.
- Item 3 is next: add a repo-wide automation-snapshot to action-catalog consistency guard for advertised action ids.
- The remaining backlog in `tmp/improvement_audit_plan.md` then covers deeper hotkey coverage, non-allowlisted file-size debt burn-down, and `app_core` native dispatch-hub splits.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Execute item 3 from `tmp/improvement_audit_plan.md`: add a repo-wide automation-snapshot to action-catalog consistency guard for advertised action ids.
2. Keep `tmp/improvement_audit_plan.md` honest as the live audit backlog and execution record for this lane.
3. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned while Phase 2 is active.
4. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
5. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (rebuilt on 2026-03-31; Phase 2 active, items 1-2 complete)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

