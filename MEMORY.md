# Agent Memory

Last Updated: 2026-04-04T12:06:07Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The user explicitly confirmed Phase 2 of the reopened runtime performance audit for the current live tree, and I am executing `tmp/perf_plan.md` strictly in order.
- The current workspace is dirty with unrelated user edits, including `docs/README.md`, `docs/plans/index.md`, and multiple controller files outside this performance lane; I must not overwrite them.
- Items 1-5 are complete in commits `3c21e5ac` (`perf(browser): split retained row state invalidation`), `362dd5bc` (`perf(browser): avoid wav page loads in row projection`), `4ee6ad01` (`perf(browser): decouple feature refresh from row projection`), `8a9ca37e` (`perf(startup): collapse hydration folder derivation`), and `43373e1f` (`perf(startup): defer initial audio device probing`).
- Item 5 now keeps persisted audio selections at startup, defers CPAL host/device/config probing until after first present, and forces the same refresh immediately if the options panel opens before the deferred flush runs.
- The latest item-5 validation lane passed focused startup-audio tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`. The latest perf-guard snapshot still comes from item 3 and reports `browser_filter_churn_latency = 3410us` p95, `browser_query_churn_latency = 62us` p95, `browser_sort_toggle_latency = 62us` p95, `hover_latency = 2296us` p95, `wheel_latency = 2442us` p95, `browser_focus_preview_latency = 51us` p95, and `browser_focus_commit_latency = 58us` p95, `waveform_interaction_latency = 216us` p95, and `waveform_pan_zoom_adjacent_latency = 168us` p95.
- `tmp/perf_plan.md` is now the live Phase 2 execution record. Item 6 remains pending: retained renderer/text allocation cleanup in `vendor/radiant`.
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Implement item 6 from `tmp/perf_plan.md`: reduce retained renderer composition churn and transient browser row text allocations in `vendor/radiant`.
2. Update `tmp/perf_plan.md`, `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md` after each completed item.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (reopened on 2026-04-04; Phase 2 in progress, items 1-5 complete in `3c21e5ac`, `362dd5bc`, `4ee6ad01`, `8a9ca37e`, and `43373e1f`)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`



