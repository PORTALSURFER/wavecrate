# Agent Memory

Last Updated: 2026-04-04T05:25:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I am executing Phase 2 of the reopened runtime performance audit for the current live tree; `tmp/perf_plan.md` is the current source of truth for the 2026-04-04 backlog.
- The current workspace is dirty with unrelated user edits in `src/app/controller/library/source_folders/actions/rename_move_delete.rs`, `src/app/controller/library/wavs/selection_ops.rs`, and `src/app/controller/tests/browser_actions/focus_navigation/commit_focus.rs`; I must not overwrite them.
- Item 1 is complete in commit `fc2fca4e` (`perf(browser): retain compact sync filter metadata`).
- The latest `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` run completed without warnings after item 1 and reports `browser_filter_churn_latency = 2421us` p95, `hover_latency = 2602us` p95, `wheel_latency = 3159us` p95, `browser_query_churn_latency = 86us` p95, `browser_focus_preview_latency = 69us` p95, `browser_focus_commit_latency = 62us` p95, `waveform_interaction_latency = 246us` p95, and `waveform_pan_zoom_adjacent_latency = 147us` p95.
- The Phase 1 baseline is still captured in `tmp/perf_plan.md`, and the latest validation record for item 1 is appended there alongside the new perf-guard results.
- `tmp/perf_plan.md` still contains 6 ROI-ranked items; item 1 is checked off and item 2 (browser feature-cache refresh key churn) is next.
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Implement item 2 from `tmp/perf_plan.md` next, then validate, update the plan, commit, and push before moving on.
2. Keep `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md` synchronized after each completed performance item.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (reopened on 2026-04-04; Phase 2 active, item 1 complete, item 2 next)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`



