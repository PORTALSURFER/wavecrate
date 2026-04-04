# Agent Memory

Last Updated: 2026-04-04T11:20:57Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The user explicitly confirmed Phase 2 of the reopened runtime performance audit for the current live tree, and I am executing `tmp/perf_plan.md` strictly in order.
- The current workspace is dirty with unrelated user edits, including `docs/README.md`, `docs/plans/index.md`, and multiple controller files outside this performance lane; I must not overwrite them.
- Items 1-2 are complete in commits `3c21e5ac` (`perf(browser): split retained row state invalidation`) and `362dd5bc` (`perf(browser): avoid wav page loads in row projection`).
- The latest item-2 validation `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` run completed without warnings and reports `browser_filter_churn_latency = 2416us` p95, `browser_query_churn_latency = 159us` p95, `browser_sort_toggle_latency = 154us` p95, `hover_latency = 2351us` p95, `wheel_latency = 2508us` p95, `browser_focus_preview_latency = 152us` p95, `browser_focus_commit_latency = 172us` p95, `waveform_interaction_latency = 207us` p95, and `waveform_pan_zoom_adjacent_latency = 175us` p95.
- `tmp/perf_plan.md` is now the live Phase 2 execution record. Items 3-6 remain pending: hot-path feature refresh decoupling, hydration churn collapse, deferred audio probing, and retained renderer/text allocation cleanup.
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Implement item 3 from `tmp/perf_plan.md`: move feature-refresh scheduling and base-stage DB revision probes out of the hot row-projection path.
2. Update `tmp/perf_plan.md`, `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md` after each completed item.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (reopened on 2026-04-04; Phase 2 in progress, items 1-2 complete in `3c21e5ac` and `362dd5bc`)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`



