# Runtime Performance Audit Plan

Date: 2026-04-04
Status: Phase 2 in progress; items 1-2 complete on 2026-04-04

## Evidence Snapshot

- `target/perf/bench.json` reports `browser_filter_churn_latency` at `2396us` p95, with `apply_stage = 24us` p95, `pull_stage = 38us` p95, and `projection_stage = 2342us` p95.
- The same snapshot reports repeated retained-model rebuild causes for interactive browser and map flows: `dirty_mask_static_rebuild_count = 24` and `bridge_model_pull_rebuild_count = 24` for filter churn, query churn, map pan proxy, and volume drag.
- The prior 2026-04-04 runtime-performance lane already removed the previous top browser-row/frame invalidation issues, so the backlog below focuses only on the remaining live bottlenecks.
- Read-only subagent audit: `Bohr` confirmed waveform render/transient work and selection/edit metadata flows as current controller-side hotspots. The renderer/runtime findings below are based on the primary audit because the vendor sweep did not return before plan cut-off.

## ROI-Ordered Backlog

### [x] 1. Keep retained app-model pulls pointer-stable for browser/map/static churn
- ROI: Very High
- Effort: L
- Expected impact: p95 interaction latency, frame time, CPU
- Evidence:
  - `target/perf/bench.json` shows `browser_filter_churn_latency.projection_stage.p95_us = 2342` while `interaction_rebuild_cause_attribution` still records `dirty_mask_static_rebuild_count = 24` and `bridge_model_pull_rebuild_count = 24` for filter churn, query churn, map pan proxy, and volume drag.
  - `src/app_core/native_bridge/projection_cache/segment_materialize.rs:20` rebuilds the retained snapshot whenever `app_key` changes, then always refreshes non-segment fields through `refresh_non_segment_static_fields` and `refresh_non_segment_overlay_fields` at `src/app_core/native_bridge/projection_cache/segment_materialize.rs:72` and `src/app_core/native_bridge/projection_cache/segment_materialize.rs:76`.
  - `src/app_core/native_shell/app_model.rs:65` and `src/app_core/native_shell/app_model.rs:146` still assemble fresh top-level model fields during full projection.
- Recommended change: split retained top-level model identity from segment-local browser/map/static metadata so `resolve_or_project_with_derived` can reuse the existing `Arc<NativeAppModel>` when only segment payloads or keyed overlay fields change; add dedicated cache keys for non-segment overlay/static groups instead of tying them to the full `app_key`.
- Risk/tradeoffs: architectural change touching retained projection invariants and tests; easy to regress segment dirtiness if keys are too coarse.
- Visual impact: None
- Completed: 2026-04-04 (`vendor/radiant` `e5c91739`, root `3c91fbef`)
- Validation plan: extend retained projection cache tests for pointer reuse and dirty-segment accuracy; rerun `scripts/run_perf_guard.ps1`, `scripts/ci_agent.ps1`, and targeted native-bridge projection tests.

### [x] 2. Stop wasting CPU on superseded waveform renders and inline transient detection
- ROI: Very High
- Effort: M
- Expected impact: startup, p95 interaction latency, frame time, CPU
- Evidence:
  - `src/app/controller/library/wavs/waveform_rendering/render_apply.rs:194` queues a new async raster job from `queue_waveform_render_now`.
  - `src/app/controller/library/wavs/waveform_rendering/render_apply.rs:274` overwrites only one `pending_waveform_render` record, so older in-flight jobs can still complete after newer requests were queued.
  - `src/app/controller/library/background_jobs/polling/runtime_handlers.rs:72` discards stale results only after those jobs already spent worker CPU.
  - `src/app/controller/library/wavs/waveform_rendering/render_apply.rs:32`, `src/app/controller/library/wavs/waveform_rendering/render_apply.rs:65`, and `src/app/controller/library/wavs/waveform_rendering/render_apply.rs:305` still run transient detection synchronously when a waveform loads without cached markers.
- Recommended change: make waveform rendering latest-only per `WaveformRenderKey` before dispatch, cancel or skip superseded work in the job lane, and move transient detection onto the same retained async pipeline or a sibling cache keyed by waveform cache token.
- Risk/tradeoffs: cancellation and replacement logic must not drop the final repaint; transient markers may appear slightly later on cold loads.
- Visual impact: Minimal
- Completed: 2026-04-04 (`root` `dacfedac`)
- Validation plan: add tests for latest-only waveform request replacement and transient-cache reuse; manually exercise rapid pan/zoom/loading; rerun `scripts/ci_agent.ps1` and perf guard.

### [ ] 3. Remove full-source path and embedding scans from loaded-similarity workflows
- ROI: High
- Effort: L
- Expected impact: p95 interaction latency, memory, CPU
- Evidence:
  - `src/app/controller/library/wavs/similar/query.rs:41` and `src/app/controller/library/wavs/similar/query.rs:126` collect all wav entry paths into a fresh `Vec<PathBuf>`.
  - `src/app/controller/library/wavs/similar/background.rs:118` and `src/app/controller/library/wavs/similar/background.rs:210` clone the full entry-path list again for follow-loaded background refresh.
  - `src/app/controller/library/wavs/similar/loaded.rs:30`, `src/app/controller/library/wavs/similar/loaded.rs:91`, `src/app/controller/library/wavs/similar/loaded.rs:100`, and `src/app/controller/library/wavs/similar/loaded.rs:149` rebuild a path lookup and score every embedding row by decoding blobs for each candidate.
  - `src/app/controller/library/wavs/browser_pipeline/helpers.rs:161` and `src/app/state/browser/search.rs:187` / `src/app/state/browser/search.rs:193` add extra transient similarity fingerprint and index lookup work on top of the full scan.
- Recommended change: retain a source-revision keyed path-to-index map and similarity score store, avoid cloning full path vectors per query, and add candidate pruning or persisted nearest-neighbor lookup so loaded-similarity refresh does not decode every embedding row each time.
- Risk/tradeoffs: biggest change in the plan; must preserve exact ordering semantics and model-version invalidation behavior.
- Visual impact: None
- Validation plan: add deterministic similarity-order tests, cache invalidation tests across source revisions/model versions, and benchmark the loaded-similarity query path before and after.

### [ ] 4. Replace rebuild-heavy selection path scans with retained selected-index lookup state
- ROI: High
- Effort: M
- Expected impact: p95 interaction latency, CPU
- Evidence:
  - `src/app/controller/library/wavs/browser_actions/selection/paths.rs:44`, `src/app/controller/library/wavs/browser_actions/selection/paths.rs:81`, and `src/app/controller/library/wavs/browser_actions/selection/paths.rs:139` repeatedly resolve selected paths back to entry indices.
  - `src/app/controller/library/wavs/browser_lists/mod.rs:48`, `src/app/controller/library/wavs/browser_lists/mod.rs:58`, and nearby selection-pruning code repeatedly call `wav_index_for_path`.
  - `src/app/controller/library/wavs/entry_access.rs:73` can fall back to source DB lookup when the retained path/index lookup cache misses.
- Recommended change: keep a retained selected-path set plus selected-index cache keyed by source/projection revisions, and make selection-pruning/action-path builders operate on set membership and cached indices instead of repeated vector scans and fallback lookups.
- Risk/tradeoffs: selection caches must invalidate correctly on source revision changes and destructive edits.
- Visual impact: None
- Validation plan: add selection-pruning and multi-select action tests across source reloads and deletes; rerun browser action test lanes plus `scripts/ci_agent.ps1`.

### [ ] 5. Deduplicate optimistic metadata mutation batches and loaded-audio membership checks
- ROI: High
- Effort: M
- Expected impact: p95 interaction latency, memory, CPU
- Evidence:
  - `src/app/controller/library/browser_controller/actions/metadata.rs:139` builds grouped BPM contexts, rollback payloads, and path collections during `apply_bpm_contexts`.
  - `src/app/controller/library/browser_controller/actions/metadata.rs:200` then forwards new vectors into `queue_metadata_mutation`.
  - `src/app/controller/library/wavs/metadata_async.rs:15` batches those operations again into pending mutation state.
  - `src/app/controller/library/wavs/metadata_async.rs:227` reopens the source DB to apply the worker batch, while optimistic checks also use linear path membership in the controller path list.
- Recommended change: introduce one retained metadata mutation batch object that carries deduped paths, optimistic cache deltas, rollback data, and worker operations once; use set membership for loaded-audio checks instead of `Vec::contains`.
- Risk/tradeoffs: rollback correctness matters more than raw speed here; the batch object must stay easy to reason about.
- Visual impact: None
- Validation plan: extend metadata mutation success/rollback tests, especially mixed BPM/no-op batches and loaded-audio optimistic refresh cases.

### [ ] 6. Reuse background workers for destructive selection edits and folder/file operations
- ROI: Medium
- Effort: M
- Expected impact: p95 interaction latency, CPU, I/O
- Evidence:
  - `src/app/controller/library/selection_edits/background.rs:109` starts a fresh thread per selection edit and reopens the source DB at `src/app/controller/library/selection_edits/background.rs:163`.
  - `src/app/controller/library/browser_controller/helpers.rs:297` and `src/app/controller/library/browser_controller/helpers.rs:367` do the same for browser-controller file operations.
  - `src/app/controller/library/source_folders/actions/rename_move_delete.rs:80`, `src/app/controller/library/source_folders/actions/rename_move_delete.rs:143`, `src/app/controller/library/source_folders/actions/rename_move_delete.rs:180`, `src/app/controller/library/source_folders/actions/rename_move_delete.rs:465`, and `src/app/controller/library/source_folders/actions/rename_move_delete.rs:547` still pay repeated thread-spawn and DB-open cost before the actual filesystem work.
- Recommended change: route these operations through a bounded reusable worker lane owned by runtime jobs, and pass already-known metadata into worker tasks when correctness allows instead of reopening and rereading it for each operation.
- Risk/tradeoffs: must preserve current error propagation and sequencing guarantees for file and database mutations.
- Visual impact: None
- Validation plan: add tests for serial ordering and failure rollback in edit/file-op workflows; manually exercise multi-item destructive edits; rerun `scripts/ci_agent.ps1`.

### [ ] 7. Stop rebuilding browser search and playback-age caches from whole-source state
- ROI: Medium
- Effort: M
- Expected impact: startup, p95 interaction latency, memory, CPU
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs:18` and `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs:35` compute playback-age cache state before visible-row reuse.
  - `src/app/controller/library/wavs/browser_pipeline/helpers.rs:177` scans all base rows to derive the playback-age cache token.
  - `src/app/controller/library/wavs/browser_search/cache.rs:65`, `src/app/controller/library/wavs/browser_search/cache.rs:145`, and the label fill path rebuild or backfill source-wide `Vec<String>` storage on demand.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages/source_cache.rs:50` rebuilds compact search entries and display labels across the whole source on revision changes.
- Recommended change: keep a retained next-expiry token for playback-age filters, incrementalize label/materialized search-entry caches by source revision, and avoid whole-source string buffer resets when only a subset of rows or metadata changed.
- Risk/tradeoffs: cache invalidation will be more complex; stale labels or missed age rollovers are the main correctness risks.
- Visual impact: None
- Validation plan: add revision-sensitive cache invalidation tests for search labels and playback-age rollover, plus perf-guard coverage for age-filter/search-heavy scenarios.

### [ ] 8. Reduce browser/runtime text allocation churn in `vendor/radiant`
- ROI: Medium
- Effort: M
- Expected impact: frame time, CPU, memory
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/frame_build/browser/panel.rs:3` still constructs toolbar button and chip labels during frame build, including `button.label.to_string()` and formatted chip labels.
  - `vendor/radiant/src/gui/native_shell/state/frame_build/overlay/focus.rs:206` rebuilds focused-row text payloads and inline-label strings during overlay rendering.
  - `vendor/radiant/src/gui_runtime/native_vello/text_runtime.rs:89` rebuilds active text-field visual state and style tokens during editor sync.
- Recommended change: move browser toolbar/focus text payload derivation into retained projection caches, reuse preformatted strings and reserved widths, and cache text-field visual state by layout/style signature instead of rebuilding it every sync.
- Risk/tradeoffs: any retained text cache must invalidate correctly on DPI/theme/layout changes; stale text bounds would show up visually.
- Visual impact: Needs review
- Validation plan: rerun focused `vendor/radiant` browser/focus/text-runtime tests, compare screenshots or manual visual checks at multiple scales, then rerun `scripts/ci_agent.ps1` and perf guard.
