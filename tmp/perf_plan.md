# Runtime Performance Audit Plan

Date: 2026-04-17
Status: Phase 2 in progress on 2026-04-17; items 1-3 complete

## Evidence Snapshot

- Fresh local guard run on 2026-04-17:
  - `target/perf/bench.json`
  - `controller_app_model_projection.p95_us = 2826`
  - `retained_app_model_projection_p95_us = 5`
  - `browser_filter_churn_latency.p95_us = 38`
  - `browser_query_churn_latency.p95_us = 65`
  - `wheel_latency.p95_us = 419`
  - `waveform_interaction_latency.p95_us = 108`
  - `feature_blob_decode.total_elapsed_ms = 5329` for `320000` blobs
- Fresh startup-profile smoke on 2026-04-17:
  - `target/perf/bench..startup_summary.json`
  - `first_present_ms = 1632.444`
  - `surface_ready_ms = 904.170`
  - `renderer_ready_ms = 1205.318`
  - `deferred_model_refresh_ms = 371.057`
- Waveform preview A/B on 2026-04-17:
  - default immediate-preview median: `target/perf/bench_default_preview.json`
    - `waveform_interaction_latency.p95_us = 158`
    - `waveform_interaction_latency.p99_us = 215`
  - preview disabled median: `target/perf/bench_preview_off.json`
    - `waveform_interaction_latency.p95_us = 1304`
    - `waveform_interaction_latency.p99_us = 13518`
  - conclusion: disabling immediate waveform preview is a regression and should not be treated as the next perf win.

## ROI-Ordered Backlog

### [x] 1. Collapse browser search-worker reload into one retained pass
- ROI: Very High
- Effort: M
- Expected impact: startup, browser refresh latency, CPU, memory
- Completed: 2026-04-17 (`e3820a2d`, `perf(browser): collapse retained worker reload passes`)
- Evidence:
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages/source_cache.rs:121`
    reloads every search row on revision changes.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages/source_cache.rs:138`
    rebuilds compact entries in a second pass.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages/source_cache.rs:152`
    hashes all paths again, and `:157` rebuilds the lookup map with fresh string clones.
  - `crates/sempal-library/src/sample_sources/db/read/file_queries.rs:284`
    and `:334` still do full ordered table scans for row and metadata refreshes.
- Recommended change:
  - Build compact entries, path fingerprint, and lookup map in one streaming reload path.
  - Tighten metadata-only refresh so the full-table metadata fallback is used only when targeted delta refresh truly cannot preserve order.
  - Avoid repeated `String` materialization for path-keyed caches where borrowed or boxed path storage already exists.
- Risk / tradeoffs:
  - Ordering parity with the DB path is correctness-sensitive; if row order drifts, retained metadata patches can silently target the wrong entries.
- Visual impact: None
- Validation plan:
  - Extend browser-search worker tests for full reloads, metadata-only refreshes, and targeted delta updates.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.

### [x] 2. Narrow sync browser-pipeline invalidation for metadata-only edits
- ROI: High
- Effort: M
- Expected impact: p95 interaction latency, CPU, allocations
- Completed: 2026-04-17 (`ef401152`, `perf(browser): narrow metadata-only pipeline invalidation`)
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline.rs:113`
    updates one compact entry in place.
  - `src/app/controller/library/wavs/browser_pipeline.rs:126`
    immediately calls `refresh_base_partitions()`.
  - `src/app/controller/library/wavs/browser_pipeline.rs:160`
    rescans every compact entry and clears downstream stage fingerprints for single-entry metadata changes.
  - `src/app/controller/library/wavs/entry_mutation/cache.rs:55`
    routes single-row metadata edits through this path.
- Recommended change:
  - Patch the affected partition buckets and stage fingerprints incrementally when tag, lock, missing, or playback-age changes affect only one entry.
  - Keep the existing full partition rebuild as a conservative fallback when multiple rows or structural mutations land together.
- Risk / tradeoffs:
  - Incorrect partial invalidation can leave stale triage partitions, sort positions, or playback-age filters.
- Visual impact: None
- Validation plan:
  - Extend sync browser-pipeline tests for single-row tag changes, lock changes, playback-age updates, and mixed fallback cases.
  - Rerun targeted browser-pipeline tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [x] 3. Split the remaining controller fallback prep lane beyond `BrowserRetainedPull`
- ROI: High
- Effort: L
- Expected impact: dirty-pull latency, frame time, CPU
- Completed: 2026-04-17 (`1e54e355`, `perf(native-bridge): split retained prep lanes`)
- Evidence:
  - `target/perf/bench.json:61` shows `controller_app_model_projection.p95_us = 2826`
    while `target/perf/bench.json:83` records `retained_app_model_projection_p95_us = 5`.
  - `src/app_core/controller.rs:190`
    still runs a broad full-prep lane that serially flushes transport, browser, metadata, waveform, startup, playhead, and governor work.
  - `src/app_core/native_bridge/runtime_projection.rs:166`
    still routes all non-local non-motion pulls through the selected prep plan before projection.
- Recommended change:
  - Introduce more specific dirty-pull maintenance lanes after `BrowserRetainedPull`, especially for metadata-only, startup-only, and transport/status-only work.
  - Keep the current full lane as the fallback when dependencies cross boundaries or multiple queues are dirty.
- Risk / tradeoffs:
  - Ordering mistakes can surface as stale status, delayed startup maintenance, or inconsistent transport/browser state.
- Visual impact: Minimal
- Validation plan:
  - Add controller/native-bridge tests for each lane boundary and mixed-dirty fallback behavior.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [ ] 4. Stop active text fields from shaping the same text twice per rebuild
- ROI: High
- Effort: M
- Expected impact: p95 typing latency, CPU, allocations
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/text_edit.rs:176`
    shapes the full text to derive stops and scroll state.
  - `vendor/radiant/src/gui_runtime/native_vello/text_edit.rs:211`
    immediately reshapes the visible slice again.
  - `vendor/radiant/src/gui_runtime/native_vello/text_runtime.rs:114`
    builds this layout on active text-field misses, and `:372` only caches the final visible payload.
- Recommended change:
  - Separate full-text shaping from visible-window extraction so caret math can reuse one shaped layout per text/editor state.
  - Cache visible-slice geometry as a derived view of the shaped full-text layout instead of running a second `layout_text` call.
- Risk / tradeoffs:
  - Caret, selection, and scroll-window math must remain exact or the active field can show mismatched highlight geometry.
- Visual impact: None
- Validation plan:
  - Extend native-vello text-field tests for long query typing, selection ranges, scroll window shifts, and BPM/text fields.
  - Rerun `cargo nextest run --manifest-path vendor/radiant/Cargo.toml text_runtime`.

### [ ] 5. Replace entry-sized similarity scratch with sparse/windowed lookup
- ROI: Medium
- Effort: M
- Expected impact: similarity sort latency, CPU, allocations
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline/helpers.rs:18`
    sizes `similar_lookup_scratch` to the full entry count.
  - `src/app/controller/library/wavs/browser_pipeline/helpers.rs:25`
    fills the lookup table entry-by-entry before sorting only the visible slice.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages/visible_rows.rs:189`
    mirrors the same full-entry scratch strategy in the worker path.
- Recommended change:
  - Replace the full-entry scratch vector with a sparse score map or a retained windowed lookup keyed only by currently relevant indices.
  - Keep anchor rotation and ordering semantics unchanged.
- Risk / tradeoffs:
  - Similarity ordering and anchor placement are correctness-sensitive, especially when scores are missing or equal.
- Visual impact: None
- Validation plan:
  - Add sync and worker tests for similarity sort ordering, anchor rotation, and missing-score fallback behavior.
  - Rerun targeted browser/similarity tests plus `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.

### [ ] 6. Tighten native text and frame-text caches to avoid clone-heavy hits
- ROI: Medium
- Effort: M
- Expected impact: steady-state CPU, memory churn, long-session responsiveness
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/text_renderer/cache.rs:74`
    appends a fresh queue record on every atom-cache hit, and `:88` only trims when capacity is exceeded.
  - `vendor/radiant/src/gui/native_shell/state/frame_text_cache.rs:33`
    and `:69` clone full cached browser/status payloads on hits.
  - `vendor/radiant/src/gui/native_shell/state/frame_text_cache.rs:184`
    hashes broad layout rectangles and model signatures into one invalidation key.
- Recommended change:
  - Make the atom-cache recency path bounded without unbounded queue growth on hits.
  - Rework browser/status text caches to reuse stored payloads without full owned clones, or split them into smaller cache layers with narrower invalidation.
- Risk / tradeoffs:
  - Cache-key bugs can leave stale text or stale positions during resize and status changes.
- Visual impact: None
- Validation plan:
  - Extend native-shell cache tests for repeated-hit behavior, resize invalidation, and status/browser text updates.
  - Manually inspect status/footer/browser chrome during resize and selection churn.

### [ ] 7. Move lightweight feature metrics out of the full feature-blob decode hot path
- ROI: Medium
- Effort: L
- Expected impact: similarity latency, background prep CPU, memory
- Evidence:
  - `target/perf/bench.json:30` records `feature_blob_decode.total_elapsed_ms = 5329` for `320000` blobs.
  - `tools/bench-cli/src/bench/feature_blob_decode.rs:22`
    still measures repeated full `decode_f32_le_blob()` work as a standalone hotspot.
  - `src/app/controller/library/wavs/similar/resolve/repository.rs:146`
    and `:192` still read `vec_blob` for batch feature-metric lookups.
- Recommended change:
  - Persist or cache lightweight metrics needed for similarity filtering/reranking separately from the full feature blob, or add a retained decode cache for repeated batch access.
  - Keep the full blob for rerank paths that genuinely need the whole vector.
- Risk / tradeoffs:
  - Schema changes and cache lifetime need careful compatibility handling across existing feature versions.
- Visual impact: None
- Validation plan:
  - Add parity tests between persisted lightweight metrics and full-blob derived values.
  - Rerun targeted similarity tests and compare `feature_blob_decode` plus similarity scenarios before/after.

### [ ] 8. Re-measure and retune hidden-startup reveal policy for current first-present cost
- ROI: Medium
- Effort: M
- Expected impact: startup, perceived responsiveness
- Evidence:
  - `target/perf/bench..startup_summary.json` reports `first_present_ms = 1632.444`,
    `surface_ready_ms = 904.170`, and `deferred_model_refresh_ms = 371.057`.
  - `vendor/radiant/src/gui_runtime/native_vello.rs:87`
    hard-codes `STARTUP_REVEAL_STALL_TIMEOUT` to `300ms`.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_startup/policy.rs:49`
    arms the hidden-window reveal deadline with that constant.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_render/present.rs:41`
    only force-reveals the window after that stall fallback expires.
- Recommended change:
  - Collect a small startup matrix across the current policy and a more adaptive reveal deadline tied to first-scene readiness or backend behavior.
  - Prefer a conservative adaptive policy over simply shortening the timeout globally.
- Risk / tradeoffs:
  - Over-eager reveal can expose placeholder pop-in or partially populated chrome.
- Visual impact: Needs review
- Validation plan:
  - Repeat startup captures with `SEMPAL_PERF_GUARD_STARTUP_PROFILE=1`.
  - Manually review sandbox launches for pop-in, text stability, and first-interaction feel.
  - Rerun vendor startup tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

## Rejected Ideas

- `[-]` Disable `SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW` to cut waveform latency.
  - Why it was considered: a one-run smoke showed lower `waveform_interaction_latency`.
  - Why it was rejected: the repeated A/B went the other direction. On 2026-04-17, default preview recorded `waveform_interaction_latency.p95_us = 158` and `p99_us = 215` in `target/perf/bench_default_preview.json`, while preview-off regressed to `p95_us = 1304` and `p99_us = 13518` in `target/perf/bench_preview_off.json`.
  - Missing evidence: none; current local measurement is strong enough to keep this out of Phase 2.

## Notes

- Previous 2026-04-17 Phase 2 runtime-performance work is preserved in git history and should remain the baseline context for the follow-up items above.
- Manual visual review is still mandatory for any startup-reveal policy change.
