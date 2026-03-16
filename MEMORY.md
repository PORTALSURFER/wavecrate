# Agent Memory

Last Updated: 2026-03-16T22:05:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is a fresh evidence-driven improvement audit backlog.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- `tmp/improvement_audit_plan.md` now records a refreshed Phase 1 ROI-ranked backlog for the live tree.
- The four conservative open-question follow-ups are codified in code comments/docs, but the ranked backlog itself is still pending explicit confirmation.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for explicit user confirmation before implementing any ranked backlog item from `tmp/improvement_audit_plan.md`.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned around the refreshed Phase 1 audit backlog in `tmp/improvement_audit_plan.md`.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Treat `scripts/ci_quick.ps1` as the default pre-push validation gate on Windows and `scripts/ci_local.ps1` as the broader parity baseline for validation/tooling changes.

## Work Notes

- Active audit execution record: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

