# Agent Memory

Last Updated: 2026-04-04T22:05:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The rebuilt runtime performance lane is complete.
- The current workspace is dirty with unrelated user edits, including `docs/README.md`, `docs/plans/index.md`, and multiple controller files outside this performance lane; I must not overwrite them.
- `tmp/perf_plan.md` is now the active Phase 2 execution record for the rebuilt runtime-performance lane.
- Item 1 is complete in `vendor/radiant` commit `e5c91739` (`perf(app): retain projected rows across model clones`) and root commit `3c91fbef` (`perf(app_core): retain projected row collections`).
- Item 1 now keeps browser, source, and folder row collections behind retained shared vectors and stores browser row text in shared `Arc<str>` buffers so top-level app-model clones no longer copy those payloads on browser/map/static churn.
- The latest perf-guard snapshot after item 1 reports `browser_filter_churn_latency = 2132us` p95 and `projection_stage = 2098us` p95 in `target/perf/bench.json`, down from the Phase 1 audit snapshot of `2396us` and `2342us`.
- Item 2 is complete in root commit `dacfedac` (`perf(waveform): drop superseded render and transient work`).
- Item 2 now drops stale waveform render work before and after rasterization, defers uncached transient detection off the UI thread, and cancels pending waveform worker state when loads or clears supersede old work.
- The latest perf-guard snapshot after item 2 stays warning-free with `browser_filter_churn_latency = 3333us` p95, `hover_latency = 2281us` p95, `wheel_latency = 2603us` p95, `browser_focus_preview_latency = 51us` p95, `browser_focus_commit_latency = 57us` p95, and `waveform_interaction_latency = 187us` p95.
- Item 3 is complete in root commit `d573ddeb` (`perf(similarity): reuse loaded query snapshots`).
- Item 3 now reuses the retained browser path snapshot for loaded-similarity requests, caches loaded-similarity queries by browser snapshot key and sample id, and reapplies only snapshot-aligned results so follow-loaded refreshes stop cloning full path vectors and rescanning the same embedding rows on cache hits.
- The latest perf-guard snapshot after item 3 stays warning-free with `browser_filter_churn_latency = 2258us` p95, `hover_latency = 2675us` p95, `wheel_latency = 2284us` p95, `browser_focus_preview_latency = 50us` p95, `browser_focus_commit_latency = 59us` p95, and `waveform_interaction_latency = 1529us` p95.
- Item 4 is complete in root commit `849f0cf6` (`perf(selection): retain selected index lookups`).
- Item 4 now keys the browser selected-index cache to the active source snapshot, reuses cached indices during selection pruning, and avoids re-resolving every selected path after unrelated row deletions or reindexing.
- The latest perf-guard snapshot after item 4 stays warning-free with `browser_filter_churn_latency = 2336us` p95, `hover_latency = 2274us` p95, `wheel_latency = 2554us` p95, `browser_focus_preview_latency = 180us` p95, `browser_focus_commit_latency = 179us` p95, and `waveform_interaction_latency = 205us` p95.
- Item 5 is complete in root commit `faf927d8` (`perf(metadata): dedupe async mutation path tracking`).
- Item 5 now computes metadata-mutation touched paths once, carries the deduped path set through the pending-state and worker job payload, and uses set-backed membership for grouped BPM optimistic updates so duplicate path scans stop cascading through controller and worker code.
- The latest perf-guard snapshot after item 5 stays warning-free with `browser_filter_churn_latency = 2284us` p95, `hover_latency = 2168us` p95, `wheel_latency = 2580us` p95, `browser_focus_preview_latency = 195us` p95, `browser_focus_commit_latency = 141us` p95, and `waveform_interaction_latency = 196us` p95.
- Item 6 is complete in root commit `4ac3945e` (`perf(file_ops): reuse one-shot worker lane`).
- Item 6 now routes one-shot destructive selection edits plus sample delete/rename and folder rename/delete work through a dedicated reusable file-op worker thread owned by `ControllerJobs`, so those hot paths stop spawning fresh per-operation threads while preserving the existing file-op cancel/apply pipeline.
- The latest perf-guard snapshot after item 6 stays warning-free with `browser_filter_churn_latency = 2310us` p95, `hover_latency = 2341us` p95, `wheel_latency = 2605us` p95, `browser_focus_preview_latency = 180us` p95, `browser_focus_commit_latency = 235us` p95, and `waveform_interaction_latency = 255us` p95.
- Item 7 is complete in root commit `0efad4c2` (`perf(search): retain browser cache refresh state`).
- Item 7 now caches the next playback-age filter rollover token by base snapshot and filter shape, preserves known browser label slots across in-memory rename/insert mutations, and refreshes worker search-entry metadata in place when source revisions change without changing ordered paths.
- The latest perf-guard snapshot after item 7 stays warning-free with `browser_filter_churn_latency = 2994us` p95, `browser_query_churn_latency = 158us` p95, `browser_sort_toggle_latency = 224us` p95, `hover_latency = 2939us` p95, `wheel_latency = 3277us` p95, `browser_focus_preview_latency = 154us` p95, `browser_focus_commit_latency = 150us` p95, and `waveform_interaction_latency = 202us` p95.
- Item 8 is complete in `vendor/radiant` commit `2f53bf98` (`perf(text): retain browser runtime text state`) and root commit `46a7168b` (`perf(renderer): reduce browser text churn`).
- Item 8 now reuses cached inline-tag widths and visible-row labels during browser/focus rendering, borrows overlay editor visuals instead of cloning them during overlay paint, and caches active text-field visual layouts by target/text/editor state so unchanged browser and folder editors stop relayouting every sync.
- The latest perf-guard snapshot after item 8 stays warning-free with `browser_filter_churn_latency = 2075us` p95, `browser_query_churn_latency = 176us` p95, `browser_sort_toggle_latency = 157us` p95, `hover_latency = 2809us` p95, `wheel_latency = 2511us` p95, `browser_focus_preview_latency = 142us` p95, `browser_focus_commit_latency = 151us` p95, and `waveform_interaction_latency = 1444us` p95.
- `tmp/perf_plan.md` is now the completed execution record for the 2026-04-04 runtime performance lane.
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Treat `tmp/perf_plan.md` as the completed runtime-performance execution record until the user opens a new performance lane.
2. Keep the runtime-performance work in strict ROI order unless the user redirects the lane.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.
4. Preserve the Windows PowerShell wrapper path for future validation runs in this environment.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (Phase 2 complete on 2026-04-04; items 1-8 complete)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`



