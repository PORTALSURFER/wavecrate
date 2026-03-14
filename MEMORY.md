# Agent Memory

Last Updated: 2026-03-14T16:24:20Z
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
- Backlog item 3 is complete: the stale doc links now point at the live `app_core/actions/catalog/` and `external_drag/` module trees, and the improvement-audit status references are current again.
- Backlog item 4 is complete: the file-size debt allowlist now matches the live scoped exceptions, and the obsolete top-5 split plan is retired in favor of `tmp/cleanup_audit_hotspots.md`.
- Backlog item 5 is complete: the updater facade now delegates asset naming, install-path validation, and archive download/checksum/unzip work to focused helper modules while keeping the updater API stable.
- Backlog item 6 is complete: the native-bridge projection cache now splits key types, segment lookup counters, and retained cache state into focused modules while preserving the existing projection/materialization behavior.
- Backlog item 7 is blocked: the browser-search ownership contract is still unclear, so deeper sync-vs-async consolidation remains unsafe without user direction.
- Backlog item 8 is complete: the remaining oversized controller regression catalogs are now split into behavior-grouped `playback_loop/` and `waveform_nav_cursor/` module trees.
- The highest-leverage current findings are:
  - The browser sync/async search split still needs an explicit ownership contract before deeper cleanup.
  - The currently safe improvement-audit backlog is otherwise complete.
  - The validation baseline is green through the completed audit items.
- The browser sync/async search split remains a clarification-sensitive area and is called out explicitly in `tmp/improvement_audit_plan.md`.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for user direction before reopening the blocked browser-search ownership item.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned if the audit lane is resumed.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Treat `scripts/ci_local.ps1` green as the current expected Windows local parity baseline unless a future doc update says otherwise.

## Work Notes

- Active audit backlog: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
