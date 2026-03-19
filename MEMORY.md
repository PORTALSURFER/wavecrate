# Agent Memory

Last Updated: 2026-03-19T13:17:14+01:00
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is Phase 2 execution of the refreshed evidence-driven improvement audit.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- The previous completed audit backlog is now historical input only.
- Backlog item 1 is implemented in local commit `7e6baff1` (`fix(drag-drop): workerize drop target transfers`).
- Backlog item 2 is implemented in local commit `c6b814d2` (`test(controller): cover folder move branches`).
- Backlog item 3 is implemented in local commit `91e5c30e` (`test(controller): cover source move apply-result branches`).
- Backlog item 4 is implemented locally and makes the Windows PowerShell validation wrappers fall back to direct `rustc` plus `tmp/agent_temp` when inherited `sccache` or the default temp dir is unusable; item 5 is next from `tmp/improvement_audit_plan.md`.
- Full-scan guardrails are currently green, so the old file-size-driven backlog no longer applies as written.
- The active follow-up is a dual-lane validation workflow for Windows: `scripts/ci_agent.ps1` is the reliable agent-safe lane in constrained environments, while `scripts/ci_quick.ps1` remains the broader integrated lane for humans when `cargo-nextest.exe` is allowed.
- `scripts/devcheck.ps1` and `scripts/ci_agent.ps1` are green again in this constrained Windows environment after the wrapper and temp-dir fallbacks landed.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Continue Phase 2 in strict `tmp/improvement_audit_plan.md` order with item 5 unless items 1-4 need a correction.
2. Keep `tmp/improvement_audit_plan.md`, `AGENTS.md`, `docs/plans/active/todo.md`, `docs/plans/index.md`, and this file aligned around the active execution lane.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Use `scripts/ci_agent.ps1` for agent-side validation in this constrained Windows environment, and treat `scripts/ci_quick.ps1` / `scripts/ci_local.ps1` as broader user-run confirmation lanes when `cargo-nextest.exe` is allowed.

## Work Notes

- Active audit backlog: `tmp/improvement_audit_plan.md` (refreshed 2026-03-18)
- Previous completed audit backlog: historical content recorded in Git history
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`


