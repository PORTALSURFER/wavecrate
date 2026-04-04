# Agent Memory

Last Updated: 2026-04-04T18:05:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I have completed Phase 2 of the reopened runtime performance audit for the current live tree; `tmp/perf_plan.md` is now the completed execution record for the 2026-04-04 backlog.
- The current workspace is dirty with unrelated user edits, including `docs/README.md`, `docs/plans/index.md`, and multiple controller files outside this performance lane; I must not overwrite them.
- Item 1 is complete in commit `fc2fca4e` (`perf(browser): retain compact sync filter metadata`).
- Item 2 is complete in commit `ef649778` (`perf(browser): retain feature refresh snapshots`).
- Item 3 is complete in vendor/radiant commit `d13e5f55` (`perf(runtime): narrow browser navigation invalidation`).
- Item 4 is complete in commits `ca24b6d3` (`test(controller): cover startup source hydration refresh`) and `18f8d5d5` (`perf(startup): defer source hydration follow-up`).
- Item 5 is complete in commit `9009d402` (`perf(browser): tighten retained row cache hot path`).
- Item 6 is complete in vendor/radiant commit `427e115b` (`perf(runtime): retain browser and status text payloads`) and superproject bump `53ea4684` (`chore(vendor): bump radiant for retained text caches`).
- The latest `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` run completed without warnings after item 6 and reports `browser_filter_churn_latency = 2767us` p95, `hover_latency = 2896us` p95, `wheel_latency = 2830us` p95, `browser_query_churn_latency = 158us` p95, `browser_focus_preview_latency = 153us` p95, `browser_focus_commit_latency = 143us` p95, `waveform_interaction_latency = 246us` p95, and `waveform_pan_zoom_adjacent_latency = 198us` p95.
- The Phase 1 baseline and the validated item 1-2 completion records are captured in `tmp/perf_plan.md`.
- `tmp/perf_plan.md` still contains the 6 ROI-ranked items, and all 6 are now checked off.
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Treat `tmp/perf_plan.md` as the completed runtime-performance execution record until the user opens a new performance lane.
2. Keep `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md` synchronized if the user reopens the performance lane or starts a new lane.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (reopened on 2026-04-04; Phase 2 complete and all 6 items are done)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`



