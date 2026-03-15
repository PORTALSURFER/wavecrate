# Agent Memory

Last Updated: 2026-03-15T12:42:32Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is a fresh evidence-driven improvement audit.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- `tmp/improvement_audit_plan.md` was rebuilt on `2026-03-15` from the live `20f83666` tree and is back in Phase 1.
- Phase 1 is complete and awaiting explicit implementation approval.
- The top current findings are:
  - the browser search-worker stage hub still mixes cache refresh, scoring, and visible-row construction in one file;
  - the analysis-job progress poller still mixes source discovery, aggregation, stale cleanup, and thread orchestration;
  - the hotkey registry, browser controller actions, source lifecycle controller, and interaction-options controller remain large allowlisted hubs;
  - the GUI platform still explicitly calls out missing transport, volume-drag, and map-point scenario coverage plus unresolved AIV foreground/focus hardening.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for explicit user confirmation before implementing any backlog item from `tmp/improvement_audit_plan.md`.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned while the refreshed audit backlog is awaiting approval.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Treat `scripts/ci_quick.ps1` as the default pre-push validation gate on Windows and `scripts/ci_local.ps1` as the broader parity baseline when needed.

## Work Notes

- Active audit backlog: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

