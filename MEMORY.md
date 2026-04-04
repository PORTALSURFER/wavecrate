# Agent Memory

Last Updated: 2026-04-04T12:38:17Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The user explicitly confirmed Phase 2 of the reopened runtime performance audit for the current live tree, and that execution is now complete.
- The current workspace is dirty with unrelated user edits, including `docs/README.md`, `docs/plans/index.md`, and multiple controller files outside this performance lane; I must not overwrite them.
- Items 1-6 are complete in commits `3c21e5ac` (`perf(browser): split retained row state invalidation`), `362dd5bc` (`perf(browser): avoid wav page loads in row projection`), `4ee6ad01` (`perf(browser): decouple feature refresh from row projection`), `8a9ca37e` (`perf(startup): collapse hydration folder derivation`), `43373e1f` (`perf(startup): defer initial audio device probing`), and vendor/radiant `9e2bc927` (`perf(renderer): reduce scene and row text churn`).
- Item 6 now keeps `vendor/radiant` scene composition grouped into retained state/motion aggregate scenes and caches browser row index/inline-chip payloads at projection time instead of reallocating them during every repaint.
- The completion validation lane passed focused `vendor/radiant` rendering tests, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`. The latest perf-guard snapshot reports `browser_filter_churn_latency = 2398us` p95, `browser_query_churn_latency = 63us` p95, `browser_sort_toggle_latency = 68us` p95, `hover_latency = 2751us` p95, `wheel_latency = 2273us` p95, `browser_focus_preview_latency = 58us` p95, `browser_focus_commit_latency = 64us` p95, `map_pan_proxy_latency = 73us` p95, `waveform_interaction_latency = 288us` p95, `waveform_pan_zoom_adjacent_latency = 176us` p95, `volume_drag_latency = 103us` p95, and `idle_cursor_motion_latency = 8us` p95.
- `tmp/perf_plan.md` is now the completed Phase 2 execution record for the runtime-performance lane.
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Treat `tmp/perf_plan.md` as the completed runtime-performance execution record until the user opens a new performance lane.
2. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.
3. Preserve the Windows PowerShell wrapper path for future validation runs in this environment.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (completed on 2026-04-04; items 1-6 complete with final vendor/radiant commit `9e2bc927`)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`



