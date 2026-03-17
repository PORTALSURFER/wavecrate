# Agent Memory

Last Updated: 2026-03-17T15:30:00+01:00
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is the completed refreshed evidence-driven improvement audit execution record.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- The refreshed audit backlog has been fully implemented and pushed.
- The previous merged audit backlog remains historical input only; the refreshed execution record supersedes it.
- Full-scan guardrails are currently green, so the old file-size-driven backlog no longer applies as written.
- The refreshed execution completed stale audit-metadata cleanup, folder-move decomposition/coverage, and the compute-worker suppression cleanup.
- The active follow-up is a dual-lane validation workflow for Windows: `scripts/ci_agent.ps1` is the reliable agent-safe lane in constrained environments, while `scripts/ci_quick.ps1` remains the broader integrated lane for humans when `cargo-nextest.exe` is allowed.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Treat `tmp/improvement_audit_plan.md` as the completed execution record for the refreshed audit lane.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, `docs/plans/index.md`, and this file aligned around the dual-lane validation workflow.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Use `scripts/ci_agent.ps1` for agent-side validation in this constrained Windows environment, and treat `scripts/ci_quick.ps1` / `scripts/ci_local.ps1` as broader user-run confirmation lanes when `cargo-nextest.exe` is allowed.

## Work Notes

- Active audit execution record: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`


