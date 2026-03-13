# Agent Memory

Last Updated: 2026-03-13T19:10:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` are both on local `next`.
- The working tree is clean except for ignored local artifact paths and temporary audit output files.
- The improvement audit lane is complete.
- The current source of truth for that completed lane is `tmp/improvement_audit_plan.md`.
- `tmp/improvement_audit_plan.md` now serves as both the ROI-ranked backlog record and the execution log for items 1 through 14 completed on `2026-03-13`.
- The file-size-budget hotspot that previously blocked `ci_local.ps1` has been cleared by splitting the oversized test/catalog files.
- `scripts/ci_local.ps1` is still blocked by the pre-existing `scripts/check_migration_boundary.ps1` termination behavior after it reaches the migration-boundary guardrail step.
- The earlier GUI/browser interaction fixes and desktop AIV coverage remain part of the current repository state and are background context, not the active execution lane.
- `tmp/cleanup_plan.md` remains parked after Phase 1 and still requires explicit confirmation before any cleanup implementation.
- `tmp/perf_plan.md` remains parked after its earlier performance work and should stay dormant unless the user reopens that lane.
- Future Windows sessions must not run the Bash workflow scripts; they should use only the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.
- Future development should use the `next` branch in both `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` unless the user explicitly directs otherwise.
- The branch policy is now enforced by `scripts/check_next_branch.ps1`/`.sh` plus git hooks installed by `scripts/install_agent_preflight_hooks.sh`.

## Immediate Next Actions

1. Wait for the user to choose the next lane.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned with the active lane summary.
4. If the user wants full CI parity, investigate or fix `scripts/check_migration_boundary.ps1` separately from the now-completed improvement-audit execution record.

## Work Notes

- Active improvement audit backlog: `tmp/improvement_audit_plan.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
- Perf execution record: `tmp/perf_plan.md`
- Perf redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`
