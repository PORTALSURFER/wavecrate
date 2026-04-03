# Agent Memory

Last Updated: 2026-04-03T23:20:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I have refreshed the runtime performance audit for the current live tree and written the new Phase 1 plan to `tmp/perf_plan.md`.
- `tmp/perf_plan.md` is now the source of truth for the 2026-04-03 ROI-ranked runtime performance backlog for the current tree.
- Phase 2 is active for the refreshed performance lane, and the ranked backlog in `tmp/perf_plan.md` is being implemented sequentially.
- Item 1 is complete in `vendor/radiant` commit `9fe71ec9`: the native runtime now caches hover, focus, and modal overlays independently instead of rebuilding one monolithic state-overlay scene.
- Item 2 is complete in `vendor/radiant` commit `58e5fe24`: the static native-shell frame builder now renders browser rows, toolbar geometry, and sidebar sections directly from retained caches instead of materializing fresh browser/source/folder vectors on every build.
- Item 3 is complete in commit `2bb31ea2`: the native bridge now keeps one retained `Arc<NativeAppModel>` and uses `Arc::make_mut` on projection misses so the full app model is no longer cloned unconditionally on every miss.
- Item 4 is complete in commit `8cf293b0`: the browser filter pipeline now avoids per-row `PathBuf` clones for mark checks, mark-only invalidation only keys on mark revisions when that filter is active, folder acceptance stays on borrowed paths, and mark pruning only touches currently marked paths instead of rebuilding a whole-library path set.
- Item 5 is complete in commit `7a91afd2`: browser commit focus now keeps selection and loading-placeholder state synchronous, defers history/similarity and the heavy half of audio dispatch to frame-time flushing, and guards stale deferred work so only the current committed focus can dispatch.
- The latest benchmark evidence comes from `target/perf/bench.json`, where the current item-5 state reports `browser_focus_commit_latency = 91us` p95, `browser_focus_preview_latency = 85us` p95, `browser_filter_churn_latency = 2477us` p95, `hover_latency = 2399us` p95, `wheel_latency = 2566us` p95, and `waveform_pan_zoom_adjacent_latency = 111us` p95.
- The current top ROI items are:
  - increase waveform adjacent-view cache locality instead of recomputing dense columns on pan/zoom churn
  - move feature-cache priming off the browser row-projection hot path
- `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` is green again on the live tree after restoring the missing `snap_override` benchmark action field in `tools/bench-cli/src/bench/gui/interactions/step_patterns.rs`.
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Continue with item 6 from `tmp/perf_plan.md`: increase waveform adjacent-view cache locality instead of recomputing dense columns on pan/zoom churn.
2. Keep `tmp/perf_plan.md`, `AGENTS.md`, `MEMORY.md`, `docs/plans/index.md`, and `docs/plans/active/todo.md` synchronized with the performance lane status.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (Phase 2 active; items 1-5 complete in commits `9fe71ec9`, `58e5fe24`, `2bb31ea2`, `8cf293b0`, and `7a91afd2`)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`



