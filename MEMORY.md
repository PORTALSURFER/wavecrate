# Agent Memory

Last Updated: 2026-03-14T12:25:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is the evidence-driven improvement audit execution.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- `tmp/improvement_audit_plan.md` was rebuilt on `2026-03-14` from current repository evidence and now serves as the execution record for the refreshed ROI-ranked backlog.
- Backlog item 1 is complete: the repository is rustfmt-clean again, and `scripts/ci_local.ps1` no longer stops at `cargo fmt --all -- --check`.
- Backlog item 2 is complete: the `vendor/radiant` toolbar layout regression is fixed, the affected snapshots/tests were refreshed, and `scripts/ci_local.ps1` is green end-to-end again.
- The next active item is backlog item 3: repair stale docs and handoff references left behind by recent module splits.
- The highest-leverage current findings are:
  - Several repo docs still point at deleted files or stale audit status.
  - The live file-size hotspot scan no longer matches the old top-5 split plan.
  - The browser sync/async search split still needs an explicit ownership contract before deeper cleanup.
- The browser sync/async search split remains a clarification-sensitive area and is called out explicitly in `tmp/improvement_audit_plan.md`.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Continue executing `tmp/improvement_audit_plan.md` in order, starting with item 3.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned while the audit lane is active.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Treat `scripts/ci_local.ps1` green as the current expected Windows local parity baseline unless a future doc update says otherwise.

## Work Notes

- Active audit backlog: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
