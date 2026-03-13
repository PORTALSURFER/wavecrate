# Agent Memory

Last Updated: 2026-03-13T19:10:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` are both on local `next`.
- The working tree is clean except for ignored local artifact paths.
- The active lane is an evidence-driven improvement audit of the current repository state.
- The current source of truth for that lane is the refreshed `tmp/improvement_audit_plan.md`.
- `tmp/improvement_audit_plan.md` contains the current ROI-ranked backlog, open questions, and rejected ideas rebuilt from the live tree.
- Phase 1 of the audit rerun is complete, and no Phase 2 implementation has started.
- The user must explicitly confirm before any item from `tmp/improvement_audit_plan.md` is implemented.
- The earlier GUI/browser interaction fixes and desktop AIV coverage remain part of the current repository state and are background context, not the active execution lane.
- `tmp/cleanup_plan.md` remains parked after Phase 1 and still requires explicit confirmation before any cleanup implementation.
- `tmp/perf_plan.md` remains parked after its earlier performance work and should stay dormant unless the user reopens that lane.
- Future Windows sessions must not run the Bash workflow scripts; they should use only the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.
- Future development should use the `next` branch in both `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` unless the user explicitly directs otherwise.
- The branch policy is now enforced by `scripts/check_next_branch.ps1`/`.sh` plus git hooks installed by `scripts/install_agent_preflight_hooks.sh`.

## Immediate Next Actions

1. Wait for the user to confirm whether to start Phase 2 from the refreshed `tmp/improvement_audit_plan.md`.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned with the active audit lane.
4. If Phase 2 is approved, execute the backlog in ROI order and validate with `scripts/devcheck.ps1` and `scripts/ci_quick.ps1` on Windows.

## Work Notes

- Active improvement audit backlog: `tmp/improvement_audit_plan.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
- Perf execution record: `tmp/perf_plan.md`
- Perf redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`
