# Agent Memory

Last Updated: 2026-04-04T11:38:42Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The user explicitly confirmed Phase 2 of the reopened runtime performance audit for the current live tree, and I am executing `tmp/perf_plan.md` strictly in order.
- The current workspace is dirty with unrelated user edits, including `docs/README.md`, `docs/plans/index.md`, and multiple controller files outside this performance lane; I must not overwrite them.
- Items 1-3 are complete in commits `3c21e5ac` (`perf(browser): split retained row state invalidation`), `362dd5bc` (`perf(browser): avoid wav page loads in row projection`), and `4ee6ad01` (`perf(browser): decouple feature refresh from row projection`).
- The latest item-3 validation `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` run completed without warnings and reports `browser_filter_churn_latency = 3410us` p95, `browser_query_churn_latency = 62us` p95, `browser_sort_toggle_latency = 62us` p95, `hover_latency = 2296us` p95, `wheel_latency = 2442us` p95, `browser_focus_preview_latency = 51us` p95, `browser_focus_commit_latency = 58us` p95, `waveform_interaction_latency = 216us` p95, and `waveform_pan_zoom_adjacent_latency = 168us` p95.
- `tmp/perf_plan.md` is now the live Phase 2 execution record. Items 4-6 remain pending: hydration churn collapse, deferred audio probing, and retained renderer/text allocation cleanup.
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Implement item 4 from `tmp/perf_plan.md`: collapse startup hydration path normalization and folder-derivation filesystem churn.
2. Update `tmp/perf_plan.md`, `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md` after each completed item.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (reopened on 2026-04-04; Phase 2 in progress, items 1-3 complete in `3c21e5ac`, `362dd5bc`, and `4ee6ad01`)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`



