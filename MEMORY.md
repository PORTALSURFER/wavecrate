# Agent Memory

Last Updated: 2026-03-17T11:27:56+01:00
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is a refreshed evidence-driven improvement audit backlog.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- Phase 1 of the refreshed audit is complete and awaits explicit user confirmation before any backlog implementation begins.
- The previous merged audit backlog is no longer the active queue; it was used only as historical input for the refresh.
- Full-scan guardrails are currently green, so the old file-size-driven backlog no longer applies as written.
- The refreshed backlog prioritizes stale audit metadata, integrity-sensitive folder-move seams, and one remaining analysis-worker suppression cleanup.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for explicit user confirmation before implementing any item from `tmp/improvement_audit_plan.md`.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, `docs/plans/index.md`, and this file aligned around the refreshed audit state.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Use `scripts/ci_quick.ps1` as the default Windows validation gate before any future push and `scripts/ci_local.ps1` when broader parity is needed.

## Work Notes

- Active audit backlog: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`


