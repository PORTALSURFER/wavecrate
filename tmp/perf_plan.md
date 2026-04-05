# Runtime Performance Audit Plan

Date: 2026-04-05
Status: Phase 2 in progress on 2026-04-05; item 1 complete, 6 items pending

## Evidence Snapshot

- `target/perf/bench.json` still shows the largest measured interaction costs in
  `hover_latency` (`5138us` p95), `wheel_latency` (`5106us` p95),
  `app_model_projection` (`4115us` p95), `interactive_projection` (`4008us`
  p95), and `browser_filter_churn_latency` (`2872us` p95).
- Stage attribution keeps the main hotspots projection-heavy:
  - `hover_latency.projection_stage.p95_us = 4484`
  - `wheel_latency.projection_stage.p95_us = 4336`
  - `browser_filter_churn_latency.projection_stage.p95_us = 2810`
  - `interactive_projection.projection_stage.p95_us = 3958`
- The retained bridge cache path itself is now cheap in the current artifact:
  - `retained_app_model_projection_p95_us = 8`
  - segment probe `browser_rows_window.p95_us = 8`
  - segment probe `waveform_overlay.p95_us = 28`
- `feature_blob_decode` still spends about `8538ms` decoding `320000` blobs,
  which keeps similarity/blob movement on the shortlist for direct work.
- Measurement caveat:
  - The current GUI interaction scenarios in
    `tools/bench-cli/src/bench/gui/interactions.rs` still time
    `prepare_native_frame(false)` plus `AppController::project_native_app_model`
    through the legacy controller/native-shell projection path.
  - The shipped runtime in `src/main.rs` constructs `GuiFixtureBridge`, which
    uses retained `SempalNativeBridge`.
  - The current perf guard therefore over-measures legacy full projection for
    hover/wheel/filter churn and under-measures real retained-runtime scene work.
- Startup caveat:
  - `scripts/run_perf_guard.ps1` still skips startup capture entirely.
  - The native startup summary only emits once deferred model refresh completes,
    but the default Windows/native startup path does not enable deferred first
    pull today.

## ROI-Ordered Backlog

### [x] 1. Stop loaded-similarity recomputes from scanning and decoding the full source
- ROI: Very High
- Effort: M
- Expected impact: p95 interaction latency, CPU, memory
- Completed: 2026-04-05 (`bd2b6a57`, `perf(similarity): cache loaded source snapshots`)
- Evidence:
  - `src/app/controller/library/wavs/similar/loaded.rs:132-206` loads every
    embedding row for the active model, filters by parsed `sample_id`, then
    decodes embedding and feature blobs row-by-row in `score_similarity_row`.
  - `src/app/controller/library/wavs/similar/resolve/repository.rs:104-185`
    already exposes batched sample loaders that can be extended for this path.
  - `target/perf/bench.json` reports
    `feature_blob_decode.total_elapsed_ms = 8538` for `320000` blobs.
- Recommended change:
  - Add a source-scoped candidate query for loaded similarity instead of walking
    the full `embeddings` table.
  - Batch embeddings/features for only the active candidate set and retain
    decoded vectors/metrics per source revision and model version.
  - Preserve the current missing-entry fill semantics after the narrowed query.
- Risk/tradeoffs:
  - Ranking parity, DSP rerank semantics, and missing-entry ordering must remain
    exact.
  - Retained decoded caches need explicit invalidation on source revision and
    model-version changes.
- Visual impact: None
- Validation plan:
  - Add loaded-similarity parity tests against the current resolver.
  - Add a large-source rerank benchmark or perf fixture with similarity enabled.
  - Rerun `cargo test -p sempal --lib app::controller::library::wavs::similar`
    and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [ ] 2. Replace full-source browser filter/folder/similarity rebuilds with indexed retained stages
- ROI: Very High
- Effort: L
- Expected impact: p95 interaction latency, CPU, memory
- Evidence:
  - `target/perf/bench.json` reports
    `browser_filter_churn_latency.p95_us = 2872`.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages/visible_rows.rs:242-291`
    still sorts and rebuilds visible rows after scanning the active entry set
    whenever non-default filters/similarity state are active.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/folders.rs:53-120`
    rebuilds folder acceptance over the full source-order entry list.
  - `src/app/controller/library/wavs/browser_pipeline/folder_stage.rs:3-55`
    mirrors that full acceptance-vector rebuild on the sync UI path.
- Recommended change:
  - Introduce retained bitset/index stages for rating, playback-age, marked, and
    folder acceptance so filter composition becomes intersection/union work
    instead of full rescans.
  - Reuse the same retained filter primitives for similarity-filtered and sync
    UI paths so both lanes stop rebuilding full visible vectors from scratch.
  - Keep sort/order caches separate from filter-membership caches.
- Risk/tradeoffs:
  - Incorrect invalidation can surface stale visible rows, wrong counts, or bad
    folder-negation behavior.
  - This is a larger architecture change across worker and sync browser paths.
- Visual impact: None
- Validation plan:
  - Extend worker and sync browser parity tests for folder filters, playback-age
    filters, similarity sort, and duplicate-cleanup mode.
  - Add targeted benchmarks for filter-only and folder-only churn on large
    sources.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`
    and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [ ] 3. Make search-entry refresh row-delta-aware instead of revision-wide metadata reloads
- ROI: High
- Effort: M
- Expected impact: startup, p95 interaction latency, CPU, memory
- Evidence:
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages/source_cache.rs:58-140`
    reloads on any revision change, calls `list_search_entry_metadata()` across
    the whole source for metadata-only refreshes, and falls back to
    `list_search_entry_rows()` plus path hashing for structural changes.
  - `src/app/controller/library/wavs/browser_pipeline/base_stage.rs:99-116`
    still falls back to `db.list_files()` when loaded pages are incomplete.
- Recommended change:
  - Carry row-level change information from the DB into the worker/sync caches
    so metadata-only changes patch retained entries in place.
  - Separate structural path changes from metadata changes more aggressively and
    skip whole-source path hashing when the wav-path set is unchanged.
  - Share the same delta-aware entry cache between worker and sync browser
    stages where possible.
- Risk/tradeoffs:
  - Row-identity correctness is sensitive; fallback to full rebuild must remain
    available whenever trust is lost.
  - Cross-path cache sharing increases coupling if boundaries are not kept clear.
- Visual impact: None
- Validation plan:
  - Add parity tests for retag, lock/unlock, playback-age refresh, rename,
    insert, and delete scenarios.
  - Add allocation or refresh-count assertions where practical.
  - Rerun worker browser tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [ ] 4. Split `prepare_native_frame(false)` into dirty-source maintenance lanes for retained bridge pulls
- ROI: High
- Effort: L
- Expected impact: p95 interaction latency, frame time, CPU
- Evidence:
  - `src/app_core/native_bridge/runtime_projection.rs:138-179` still calls
    `controller.prepare_native_frame(false)` whenever the local-only fast path
    does not apply.
  - `src/app_core/controller.rs:105-147` serially polls background jobs,
    flushes multiple pending writes/commits, refreshes waveform image state,
    runs startup maintenance, ticks the playhead, refreshes revisions, and
    updates the performance governor in one broad gate.
  - `target/perf/bench.json` rebuild attribution still shows
    `bridge_model_pull_rebuild_count = 24` for filter/query/sort/map/volume
    scenarios.
- Recommended change:
  - Split `prepare_native_frame` into explicit maintenance lanes keyed by dirty
    source, transport state, pending async jobs, and deadline-driven tasks.
  - Let browser-only retained pulls skip unrelated waveform/audio/startup
    maintenance when no relevant dirty state exists.
  - Keep one conservative fallback path for any ambiguous or mixed dirty state.
- Risk/tradeoffs:
  - This touches correctness-sensitive runtime plumbing; missing one dependency
    could desync playhead, waveform image, pending commits, or async job state.
- Visual impact: Needs review
- Validation plan:
  - Add native-bridge/controller tests for browser-only pulls, waveform-image
    refresh timing, pending age updates, volume commits, and startup deferred
    maintenance.
  - Rerun perf guard, bridge tests, and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [ ] 5. Retain vendor browser-row geometry and truncation per row instead of invalidating the whole visible window
- ROI: High
- Effort: M
- Expected impact: p95 interaction latency, frame time, CPU
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/cache.rs:80-119` caches browser
    rows at whole-window granularity.
  - `vendor/radiant/src/gui/native_shell/state/browser_rows/windowing/projection.rs:156-223`
    walks the window and recomputes text layout, truncation, inline-tag rects,
    label widths, and owned strings for every row on cache misses.
  - `vendor/radiant/src/gui/native_shell/state/browser_rows/truncation.rs:103-120`
    hashes all visible row labels/metadata into one text revision fingerprint.
- Recommended change:
  - Split stable per-row text/geometry caches from volatile focus/selection
    overlay state so window-focus churn does not rebuild the whole row window.
  - Reuse cached row text layouts, truncated labels, and inline-tag geometry
    when only selection/focus/anchor changes.
  - Keep the rendered-row cache keyed by row identity plus width bucket rather
    than whole-window state where feasible.
- Risk/tradeoffs:
  - Row identity and truncation invalidation must stay exact or stale text/rects
    will show up immediately.
- Visual impact: None
- Validation plan:
  - Extend vendor browser-row rendering, truncation, virtualization, and focus
    overlay tests.
  - Manually inspect dense browser rows, duplicate-cleanup labels, and focus
    transitions at multiple viewport sizes.
  - Rerun `cargo nextest run --manifest-path vendor/radiant/Cargo.toml` and the root perf guard.

### [ ] 6. Make startup first-present measurable on Windows and then shift the default path toward progressive reveal
- ROI: High
- Effort: M
- Expected impact: startup, CPU
- Evidence:
  - `scripts/run_perf_guard.ps1:233-240` explicitly skips startup capture.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_startup/policy.rs:4-13`
    keeps the window hidden by default and disables deferred first-model pull.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_startup/initialization.rs:187-195`
    marks model/layout dirty and rebuilds the first full scene during startup.
  - `vendor/radiant/src/gui_runtime/native_vello/startup.rs:50-101` only emits
    the startup summary once deferred model refresh is complete.
- Recommended change:
  - Implement Windows startup capture in the PowerShell perf guard using the
    existing native startup profiler and/or run-contract artifacts.
  - Make the startup profiler emit first-present summaries on both eager and
    deferred startup paths.
  - After measurement parity is in place, trial a default progressive startup:
    earlier placeholder/first-scene reveal with deferred heavy refresh only when
    it improves first-visible latency without obvious pop-in.
- Risk/tradeoffs:
  - Earlier reveal can expose partially-populated UI or defer visible work into
    the first interactive moments.
  - Startup policy changes affect user-perceived polish and need manual review.
- Visual impact: Needs review
- Validation plan:
  - Capture repeated startup runs on Windows before/after, recording first-scene
    and first-present timings.
  - Run GUI startup tests in `vendor/radiant` and manual sandbox startups.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [ ] 7. Align GUI perf guard scenarios with the retained runtime path and demote controller projection to diagnostic mode
- ROI: Medium
- Effort: M
- Expected impact: p95 interaction latency tracking, startup tracking
- Evidence:
  - `tools/bench-cli/src/bench/gui/scenario_registry.rs:52-137` still records
    the main GUI scenarios through `project_native_app_model()`.
  - `tools/bench-cli/src/bench/gui/interactions.rs:30-129` measures hover,
    wheel, filter, query, and sort churn by projecting the legacy controller
    native-shell model after each action.
  - `target/perf/bench.json` simultaneously reports
    `app_model_projection.p95_us = 4115` and
    `retained_app_model_projection_p95_us = 8`, which makes the main guard hard
    to interpret for the shipped runtime.
- Recommended change:
  - Add bridge-backed interaction scenarios that measure retained runtime pulls
    and scene rebuild metrics end-to-end.
  - Keep the controller projection path only as an explicit diagnostic mode in
    reports, not the default headline metric.
  - Surface startup summaries beside retained interaction results once Windows
    startup capture exists.
- Risk/tradeoffs:
  - Bench complexity and fixture parity must remain understandable so perf drift
    is interpretable over time.
- Visual impact: None
- Validation plan:
  - Add parity assertions between controller and retained bridge outputs for
    representative fixtures.
  - Rerun `cargo test -p sempal-bench-cli --bin sempal-bench gui` and
    `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.
