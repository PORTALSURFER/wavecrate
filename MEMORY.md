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
- Backlog item 1 is complete: the repository is rustfmt-clean again, and `scripts/ci_local.ps1` now reaches the later full-suite baseline failure instead of stopping at `cargo fmt --all -- --check`.
- The next active item is backlog item 2: define or repair the expected Windows full-CI baseline.
- The highest-leverage current findings are:
  - `scripts/ci_local.ps1` now reaches the later `vendor/radiant` failure `gui::native_shell::layout_adapter::controls::controls_tests::toolbar_search_field_uses_ratio_width_inside_full_host`.
  - Several repo docs still point at deleted files or stale audit status.
  - The live file-size hotspot scan no longer matches the old top-5 split plan.
- The browser sync/async search split and the intended Windows full-CI baseline both remain clarification-sensitive areas and are called out explicitly in `tmp/improvement_audit_plan.md`.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Continue executing `tmp/improvement_audit_plan.md` in order, starting with item 2.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned while the audit lane is active.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Treat the current `vendor/radiant` layout test failure as the active full-CI baseline issue until item 2 resolves it or documents it.

## Work Notes

- Active audit backlog: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
