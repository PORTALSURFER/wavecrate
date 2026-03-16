# Agent Memory

Last Updated: 2026-03-16T10:03:55Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is the completed refreshed evidence-driven improvement audit handoff.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- `tmp/improvement_audit_plan.md` now records the refreshed ROI-ranked backlog plus the finished Phase 2 execution record for the current codebase.
- The refreshed audit lane is complete; items 1-8 are done as of 2026-03-16.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for the next user-directed lane before reopening `tmp/improvement_audit_plan.md`.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned around the completed audit execution record.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Treat `scripts/ci_quick.ps1` as the default pre-push validation gate on Windows and `scripts/ci_local.ps1` as the broader parity baseline for validation/tooling changes.

## Work Notes

- Active audit backlog: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

