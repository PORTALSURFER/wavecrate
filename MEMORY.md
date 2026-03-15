# Agent Memory

Last Updated: 2026-03-15T10:50:55Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is a refreshed evidence-driven improvement audit.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- `tmp/improvement_audit_plan.md` was rebuilt on `2026-03-15` from the live `6c9dc2d8` tree and now contains a new Phase 1 ROI-ranked backlog.
- Phase 1 is complete and no implementation has started yet.
- The highest-leverage current findings are:
  - The quick validation path still omits the semantic GUI contract lane even though the GUI test platform docs say it should be promoted into `ci_quick`.
  - Browser controller state still carries duplicated selection/search projection responsibilities across `browser_search.rs`, `browser_lists.rs`, and `src/app/state/browser.rs`.
  - `wav_sanitize.rs` and deferred undo/file-op flows remain under-defined relative to their public/runtime importance.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for explicit user confirmation before implementing any item from `tmp/improvement_audit_plan.md`.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned while the audit lane is awaiting approval.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Treat `scripts/ci_local.ps1` green as the current expected Windows local parity baseline.

## Work Notes

- Active audit backlog: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

