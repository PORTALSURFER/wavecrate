# Runtime Performance Audit Plan

Status: Phase 2 complete on 2026-04-04. Items 1-6 are complete.

- Repository state audited: superproject `7d2b4dc2`, `vendor/radiant` `427e115b`
- Workspace note: the live tree is dirty with unrelated user edits; Phase 2 must avoid overwriting them.
- Fresh perf baseline: `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` with `SEMPAL_PERF_GUARD_RUNS=3`
- Median p95 snapshot from the fresh 3-run baseline:
  - `browser_filter_churn_latency = 2937us`
  - `browser_query_churn_latency = 163us`
  - `browser_sort_toggle_latency = 272us`
  - `hover_latency = 2833us`
  - `wheel_latency = 3468us`
  - `browser_focus_preview_latency = 220us`
  - `browser_focus_commit_latency = 157us`
  - `map_pan_proxy_latency = 115us`
  - `waveform_interaction_latency = 258us`
  - `waveform_pan_zoom_adjacent_latency = 163us`
  - `volume_drag_latency = 86us`
  - `idle_cursor_motion_latency = 8us`
- Supporting hotspot evidence: `target/perf/bench.json:39`, `target/perf/bench.json:83`, `target/perf/bench.json:437`, `target/perf/bench.json:527`, `target/perf/bench.json:617`, `target/perf/bench.json:707`
- Canonical validation commands for this lane:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`

## ROI-Ranked Backlog

### [x] 1. Split browser focus and selection invalidation away from full row-window projection
- ROI: Very High
- Effort: L
- Expected impact: p95 interaction latency, frame time, CPU
- Evidence:
  - `src/app_core/native_bridge/projection_cache/key_types.rs:133`
  - `src/app_core/native_bridge/projection_cache/projection_key/browser.rs:38`
  - `src/app_core/native_bridge/projection_cache/segment_materialize.rs:194`
  - `src/app_core/native_shell/browser_projection/row_window.rs:50`
  - `target/perf/bench.json:83`
  - `target/perf/bench.json:437`
- Recommended change: split ephemeral row-state changes such as focused row, anchor row, and visible selection decorations out of the coarse browser rows projection cache key, then retain and patch only the affected rows or overlays instead of rebuilding the entire row window on every focus, hover, wheel, and selection transition.
- Risk/tradeoffs: this changes invalidation boundaries and retained-state ownership, so stale highlight, anchor, or selection visuals are the main regression risk. The implementation needs targeted tests around row focus, range selection, and offscreen selection indicators.
- Visual impact: Needs review
- Validation plan:
  - Add projection-cache tests covering focus-only, anchor-only, and selection-decoration-only transitions.
  - Extend browser cache and focus-navigation tests to assert row text and highlight correctness.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` and compare browser interaction p95s against the baseline.
- Completion record: 2026-04-04, commit `3c21e5ac`
- Validation result:
  - Focus and selection-only row-state changes now reuse retained row content and patch only `selected`/`focused` flags in place.
  - `projection_segment_browser*` targeted tests passed.
  - `browser_rows_state_patch_updates_flags_without_rebuilding_labels` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` passed with `browser_filter_churn_latency = 2617us` p95, `hover_latency = 4288us` p95, `wheel_latency = 3169us` p95, `browser_focus_preview_latency = 179us` p95, and `browser_focus_commit_latency = 228us` p95.

### [x] 2. Remove UI-thread wav page loads from browser row projection and BPM preload
- ROI: High
- Effort: M
- Expected impact: p95 interaction latency, CPU, I/O
- Evidence:
  - `src/app_core/native_shell/browser_projection/cache.rs:151`
  - `src/app_core/native_shell/browser_projection/cache.rs:152`
  - `src/app_core/native_shell/browser_projection/preload.rs:122`
  - `src/app_core/native_shell/browser_projection/preload.rs:123`
  - `src/app/controller/library/wavs/entry_access.rs:157`
  - `src/app/controller/library/wavs/metadata_cache.rs:40`
  - `src/app/controller/library/wavs/browser_search/cache.rs:157`
- Recommended change: stop calling `wav_entry()` from the row projection path and preload helpers when a stable path or lightweight metadata snapshot is enough. Introduce a page-load-free browser row input path, avoid cloning row paths before cache-hit decisions, and move BPM/search label warmup onto bounded background or retained caches.
- Risk/tradeoffs: low behavioral risk, but the new lightweight path must stay consistent with paged entry ownership and cannot return stale relative paths after mutations. Tests need to cover navigation across unloaded pages and first-scroll behavior.
- Visual impact: None
- Validation plan:
  - Add tests for projecting unloaded-page rows without forcing page loads.
  - Extend browser preload and browser search cache tests for first-window exposure.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` and verify filter, hover, and wheel improvements.
- Completion record: 2026-04-04, commit `362dd5bc`
- Validation result:
  - Browser row projection, BPM preload path collection, and browser search label fill now read from the retained browser pipeline snapshot before touching paged wav entries.
  - `browser_rows_projection_uses_pipeline_snapshot_when_pages_are_unloaded` passed.
  - `label_lookup_uses_pipeline_snapshot_when_pages_are_unloaded` passed.
  - `text_query_branch_uses_search_scores_to_filter_visible_rows` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` passed with `browser_filter_churn_latency = 2416us` p95, `browser_query_churn_latency = 159us` p95, `browser_sort_toggle_latency = 154us` p95, `hover_latency = 2351us` p95, `wheel_latency = 2508us` p95, `browser_focus_preview_latency = 152us` p95, `browser_focus_commit_latency = 172us` p95, `waveform_interaction_latency = 207us` p95, and `waveform_pan_zoom_adjacent_latency = 175us` p95.

### [x] 3. Move feature-refresh scheduling and base-stage DB revision probes out of the hot row-projection path
- ROI: High
- Effort: M
- Expected impact: p95 interaction latency, startup follow-up latency, CPU
- Evidence:
  - `src/app_core/native_shell/browser_projection/row_window.rs:52`
  - `src/app/controller/library/wavs/feature_cache.rs:260`
  - `src/app/controller/library/wavs/browser_pipeline.rs:112`
  - `src/app/controller/library/wavs/browser_pipeline.rs:115`
  - `src/app/controller/library/wavs/browser_pipeline/base_stage.rs:4`
  - `src/app/controller/library/wavs/browser_pipeline/base_stage.rs:8`
  - `src/app/controller/library/wavs/browser_pipeline/base_stage.rs:9`
- Recommended change: decouple feature-cache refresh scheduling from every row-window projection, cache the base-stage fingerprint from existing mutation boundaries, and avoid probing `database_for(...).get_revision()` on hot interaction reads when no source revision change has occurred.
- Risk/tradeoffs: the main risk is delayed feature-cache freshness after source edits or hydration. The change needs explicit ownership for when a source revision invalidates the base stage and when a browser refresh may safely reuse it.
- Visual impact: None
- Validation plan:
  - Add browser pipeline tests for source revision changes, selection-only interaction, and feature-cache refresh triggering.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.
- Completion record: 2026-04-04, commit `4ee6ad01`
- Validation result:
  - Browser row-window rendering no longer queues feature-cache refresh work; refresh scheduling now happens when visible-row projections are applied, while base-stage reuse trusts explicit pipeline invalidation instead of probing the source DB revision on each rebuild.
  - Same-path tag, loop, playback-age, rollback, and test-entry mutations now invalidate the retained browser pipeline so cached compact rows stay fresh.
  - `base_stage_reuses_cached_fingerprint_without_rechecking_db_revision` passed.
  - `base_stage_rebuilds_after_same_path_tag_updates` passed.
  - `rebuild_browser_lists_queues_feature_cache_refresh` passed.
  - `browser_rows_projection_does_not_queue_feature_cache_refresh` passed.
  - `browser_feature_cache_refresh_updates_row_metadata` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` passed with `browser_filter_churn_latency = 3410us` p95, `browser_query_churn_latency = 62us` p95, `browser_sort_toggle_latency = 62us` p95, `hover_latency = 2296us` p95, `wheel_latency = 2442us` p95, `browser_focus_preview_latency = 51us` p95, `browser_focus_commit_latency = 58us` p95, `waveform_interaction_latency = 216us` p95, and `waveform_pan_zoom_adjacent_latency = 168us` p95.

### [x] 4. Collapse startup hydration path normalization and folder-derivation filesystem churn
- ROI: High
- Effort: M
- Expected impact: startup, first interaction latency, CPU, I/O, memory
- Evidence:
  - `src/app/controller/library/sources/hydration/worker.rs:58`
  - `src/app/controller/library/sources/hydration/worker.rs:80`
  - `src/app/controller/library/sources/hydration/worker.rs:89`
  - `src/app/controller/library/sources/hydration/worker.rs:95`
  - `src/app/controller/library/sources/hydration/worker.rs:102`
  - `src/app/controller/config.rs:148`
- Recommended change: reuse the loaded entry list to build path lookup and available folders in one pass, avoid allocating normalized `PathBuf` copies with `to_string_lossy().replace(...)`, and stop doing repeated `join(...).is_dir()` checks for every ancestor during hydration follow-up when the source tree shape can be derived from the entry set.
- Risk/tradeoffs: low UI risk, but folder-tree correctness across mixed separators, empty folders, and deleted ancestors must stay exact. The optimized path should preserve current source-folder behavior on Windows paths.
- Visual impact: None
- Validation plan:
  - Add hydration worker tests for Windows-style separators, nested folders, and deleted-folder reconciliation.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.
  - Run a startup smoke check with `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`.
- Completion record: 2026-04-04, commit `8a9ca37e`
- Validation result:
  - Source hydration now builds lookup keys, entry-path clones, and folder availability from one retained entry pass, while folder validation happens once per unique folder instead of once per entry ancestor.
  - Deferred folder refresh now also batches ancestor validation after collection instead of probing `is_dir()` inside the per-entry walk.
  - `hydration_entry_maps_build_lookup_and_folders_in_one_pass` passed.
  - `hydration_entry_maps_skip_folder_derivation_for_deferred_follow_up_work` passed.
  - `hydration_entry_maps_filter_missing_folder_ancestors_once_per_unique_path` passed.
  - `startup_active_source_hydration_defers_follow_up_work_after_first_paint` passed.
  - `toggle_show_all_folders_keeps_previous_rows_while_projection_is_pending` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed.

### [x] 5. Defer expensive audio device probing until after first present or explicit settings access
- ROI: Medium
- Effort: S
- Expected impact: startup, first interaction latency
- Evidence:
  - `src/app/controller/config.rs:92`
  - `src/app/controller/config.rs:93`
  - `src/app/controller/playback/audio_options/controller.rs:82`
  - `src/app/controller/playback/audio_options/controller.rs:92`
  - `src/audio/output/discovery.rs:10`
  - `src/audio/output/discovery.rs:56`
  - `src/audio/input/enumerate.rs:9`
  - `src/audio/input/enumerate.rs:56`
- Recommended change: stop enumerating hosts, devices, and supported configurations synchronously during initial config apply. Populate cached defaults cheaply at startup, then refresh the full audio option model after first present or when the user opens playback/input settings.
- Risk/tradeoffs: minimal UX risk if the deferred refresh is visible for too long or if settings open before the async probe completes. The first settings-open experience must still show deterministic loading and preserve the current selected device when available.
- Visual impact: Minimal
- Validation plan:
  - Add controller tests for startup config apply without immediate device probing.
  - Add playback settings tests that ensure deferred refresh eventually populates host and device lists.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.
- Completion record: 2026-04-04, commit `43373e1f`
- Validation result:
  - Startup config apply now preserves the persisted audio selections but clears probed host/device/rate detail and arms a controller-owned deferred refresh that runs after first present.
  - Opening the options panel forces the deferred refresh immediately if it has not already run, so explicit settings access still sees the probed audio model without waiting for a later frame.
  - `apply_configuration_arms_deferred_startup_audio_refresh` passed.
  - `startup_audio_refresh_waits_until_after_first_prepare` passed.
  - `open_options_menu_flushes_deferred_startup_audio_refresh_once` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed.

### [x] 6. Reduce retained renderer composition churn and transient browser row text allocations
- ROI: Medium
- Effort: M
- Expected impact: frame time, CPU, memory
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_render/scene/composition.rs:305`
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_render/scene/composition.rs:306`
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_render/scene/composition.rs:307`
  - `vendor/radiant/src/gui/native_shell/state/frame_build/browser/rows.rs:129`
  - `vendor/radiant/src/gui/native_shell/state/frame_build/browser/rows.rs:207`
  - `vendor/radiant/src/gui/native_shell/state/frame_build/browser/rows.rs:224`
- Recommended change: keep retained scene composition incremental when only one overlay layer changes, and cache or borrow row number, label, and inline-chip text payloads instead of allocating fresh `String`s during every browser frame build.
- Risk/tradeoffs: retained-scene invalidation bugs can cause missing overlays or stale row text, so this item needs visual verification on hover, focus, modal, and browser row metadata states.
- Visual impact: Needs review
- Validation plan:
  - Extend `vendor/radiant` runtime scene-cache tests around overlay-only invalidation.
  - Add browser row rendering tests for labels, inline chips, and focus overlays.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.
- Completion record: 2026-04-04, commit `9e2bc927`
- Validation result:
  - `vendor/radiant` now retains grouped `state_overlay_scene` and `motion_overlay_scene` aggregates, so final Vello scene composition only appends three layers after overlay-only changes instead of rebuilding a six-layer append chain every time.
  - Browser row projection now caches `visible_row_label`, `inline_tag_labels`, and `inline_tag_rects`, so row rendering reuses projected metadata geometry instead of splitting and reallocating inline chip payloads on each repaint.
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_inline_metadata` passed.
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_row_label_truncation_uses_slotized_sample_width` passed.
  - `cargo test --manifest-path vendor/radiant/Cargo.toml startup_placeholder_scene_uses_theme_clear_color_and_branding` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` passed with `browser_filter_churn_latency = 2398us` p95, `browser_query_churn_latency = 63us` p95, `browser_sort_toggle_latency = 68us` p95, `hover_latency = 2751us` p95, `wheel_latency = 2273us` p95, `browser_focus_preview_latency = 58us` p95, `browser_focus_commit_latency = 64us` p95, `map_pan_proxy_latency = 73us` p95, `waveform_interaction_latency = 288us` p95, `waveform_pan_zoom_adjacent_latency = 176us` p95, `volume_drag_latency = 103us` p95, and `idle_cursor_motion_latency = 8us` p95.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed.

## Notes

- Architectural follow-up considered but intentionally left out of the Phase 2 starter queue: similarity-query setup still walks all wav entries (`src/app/controller/library/wavs/similar/query.rs:126`, `src/app/controller/library/wavs/entry_access.rs:86`). It is real, but it is lower ROI than the six items above because it does not dominate the current startup or interaction baseline.
- Phase 2 execution on 2026-04-04 is complete. Keep this file as the finished runtime-performance execution record until the user opens a new performance lane.
