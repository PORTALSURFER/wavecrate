# Runtime Performance Audit Plan

Date: 2026-04-17
Status: Phase 2 in progress on 2026-04-17; items 1-6 complete, item 7 next

## Evidence Snapshot

- Fresh local guard run on 2026-04-17 after item 3:
  - `target/perf/bench.json`
  - `hover_latency.p95_us = 261`
  - `wheel_latency.p95_us = 508`
  - `browser_filter_churn_latency.p95_us = 46`
  - `browser_query_churn_latency.p95_us = 53`
  - `app_model_projection.p95_us = 5`
  - `controller_app_model_projection.p95_us = 2708`
  - `retained_app_model_projection_p95_us = 5`
  - The guard completed without warnings.
- The retained bridge path itself remains cheap in the same artifact:
  - `interaction_segment_attribution.browser_rows_window.p95_us = 11`
  - `interaction_segment_attribution.waveform_overlay.p95_us = 29`
- A reduced startup-profile smoke run on Windows now captures the native-vello summary:
  - `first_present_ms = 2377.742`
  - `deferred_model_refresh_ms = 0.000`
  - `scripts/run_perf_guard.ps1` emitted the startup summary and recommended lock thresholds.
- The repo still carries a measurable feature-blob decode hotspot:
  - `feature_blob_decode.total_elapsed_ms = 5494` for `320000` blobs

## ROI-Ordered Backlog

### [x] 1. Split `prepare_native_frame(false)` into dirty-source maintenance lanes for retained pulls
- Classification: Architecture improvement
- Confidence: High
- ROI: Very High
- Effort: L
- Expected impact: p95 interaction latency, frame time, CPU
- Completed: 2026-04-17 (`cec627fd`, `perf(native-bridge): add retained browser prep lane`)
- Evidence:
  - `target/perf/bench.json` shows `hover_latency.p95_us = 8443`,
    `interactive_projection.p95_us = 7843`, and `app_model_projection.p95_us = 4128`,
    with projection-stage time dominating the browser-heavy scenarios.
  - `src/app_core/native_bridge/runtime_projection.rs:139-167` calls
    `controller.prepare_native_frame(false)` on every non-local fast-path pull.
  - `src/app_core/controller.rs:105-147` serially polls jobs, flushes pending writes,
    refreshes waveform/startup work, ticks playback, refreshes revisions, and updates
    the performance governor inside one broad gate.
- Recommended change:
  - Break native-frame preparation into explicit lanes keyed by dirty source:
    background jobs, pending settings commits, waveform image work, startup
    maintenance, playback tick, and governor maintenance.
  - Let browser-only retained pulls run only the lanes they actually need.
  - Keep one conservative full-prep fallback for ambiguous mixed state.
- Risk / tradeoffs:
  - Correctness-sensitive ordering can affect pending commits, waveform refresh,
    startup work, or playback state if a dependency is missed.
- Visual impact: Needs review
- Validation plan:
  - Add controller/native-bridge tests for browser-only pulls, waveform image
    refresh timing, pending volume/age commits, and deferred startup work.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [x] 2. Replace sync browser filter/folder rebuilds with retained stage indexes
- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Expected impact: p95 interaction latency, CPU, allocations
- Completed: 2026-04-17 (`547a9c9b`, `perf(browser): retain filter and folder stage indexes`)
- Evidence:
  - `target/perf/bench.json` now reports `browser_filter_churn_latency.p95_us = 2856`,
    down from `4435` in the audit snapshot.
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs:164-223`
    rescans `base_rows` and rebuilds `filtered_rows` entry-by-entry whenever the
    filtered-stage fingerprint changes.
  - `src/app/controller/library/wavs/browser_pipeline/folder_stage.rs:4-55`
    rebuilds a full `Vec<bool>` acceptance map for the whole source snapshot.
  - The sync controller projection path is still the default benchmark headline path in
    `tools/bench-cli/src/bench/gui/interactions.rs:30-328`.
- Recommended change:
  - Port the worker-side retained filter/folder primitives into the sync browser
    pipeline, or share the same index/cache ownership where boundaries allow it.
  - Retain accepted-row sets and sorted positions by metadata/folder revisions
    instead of rescanning the whole source for filter-only churn.
  - Keep sort-order caches separate from acceptance caches.
- Risk / tradeoffs:
  - Stale visible rows or wrong folder-negation behavior will show immediately if
    invalidation is not exact.
- Visual impact: None
- Validation plan:
  - Extend sync browser pipeline tests for folder filters, marked-only,
    playback-age filters, similarity sort, and duplicate-cleanup mode.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [x] 3. Make the perf guard measure the shipped retained runtime and capture Windows startup summaries
- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: M
- Expected impact: startup tracking, p95 interaction latency tracking, engineering time
- Completed: 2026-04-17 (`140a8640`, `perf(bench): measure retained runtime in guard`; vendor `174aa295`, `perf(native-vello): emit eager startup summaries`)
- Evidence:
  - Fresh guard output shows `app_model_projection.p95_us = 4128` but
    `retained_app_model_projection_p95_us = 15`, proving the headline metric is
    not the shipped runtime path.
  - `tools/bench-cli/src/bench/gui/interactions.rs:46-324` and
    `tools/bench-cli/src/bench/gui/scenario_registry.rs:98-136` still measure
    controller projection directly.
  - `scripts/run_perf_guard.ps1:240` explicitly warns that startup profiling is not
    implemented on Windows.
  - `vendor/radiant/src/gui_runtime/native_vello/startup.rs:50-102` only emits the
    startup summary once the deferred refresh path completes today.
- Recommended change:
  - Add bridge-backed GUI scenarios that report retained runtime pull + rebuild metrics
    as the default headline numbers and keep controller projection as diagnostic output.
  - Make startup summaries emit on both eager and deferred startup paths.
  - Teach `scripts/run_perf_guard.ps1` to collect and validate startup summary output.
- Risk / tradeoffs:
  - Bench output becomes richer and needs careful report labeling so controller-mode and
    retained-runtime numbers are not conflated.
- Visual impact: None
- Validation plan:
  - Add parity assertions between controller and retained bridge outputs for fixture runs.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.
  - Run `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [x] 4. Add a true metadata-only DB path for browser search refreshes
- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: S
- Expected impact: startup, metadata refresh latency, allocations
- Completed: 2026-04-17 (`caf5d4cb`, `perf(db): add metadata-only search refresh query`)
- Evidence:
  - `crates/sempal-library/src/sample_sources/db/read/file_queries.rs:311-318`
    implements `list_search_entry_metadata()` by calling `list_search_entry_rows()`
    and then discarding paths.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages/source_cache.rs:103-114`
    uses that metadata-only call for retained-cache refreshes.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages/source_cache.rs:171-188`
    only needs tag, lock, and playback-age fields to patch cached entries in place.
- Recommended change:
  - Add a dedicated `SELECT tag, locked, last_played_at ... ORDER BY path ASC`
    query that skips relative-path decoding/allocation.
  - Use that path for metadata-only refreshes and keep the full-row fallback for
    structural changes and trust loss.
- Risk / tradeoffs:
  - Ordering must stay identical to the full-row query or retained metadata patches
    will drift silently.
- Visual impact: None
- Validation plan:
  - Add DB read tests for metadata ordering parity.
  - Extend browser search worker tests for tag/lock/playback-age refreshes.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

### [x] 5. Stop duplicate/similarity RMS checks from decoding whole feature blobs
- Classification: Bug fix
- Confidence: High
- ROI: Medium
- Effort: M
- Expected impact: similarity latency, CPU, memory
- Completed: 2026-04-17 (`a384984a`, `perf(similarity): avoid full feature decodes for rms checks`)
- Evidence:
  - `target/perf/bench.json` still reports `feature_blob_decode.total_elapsed_ms = 5494`
    for `320000` blobs.
  - `src/app/controller/library/wavs/similar/resolve/mod.rs:45-49` checks duplicate-mode
    silence by calling `load_rms_for_sample()`.
  - `src/app/controller/library/wavs/similar/resolve/ranking.rs:123-162` loads RMS for
    duplicate filtering through `load_feature_metrics_for_samples()`.
  - `src/app/controller/library/wavs/similar/resolve/repository.rs:149-191` decodes each
    full `vec_blob` into `Vec<f32>` before reading `rms` and deriving `light_dsp`.
- Recommended change:
  - Add a lightweight feature-metric reader that extracts RMS directly from the blob bytes
    without allocating the full vector.
  - Derive `light_dsp` only on the rerank path that actually needs it, and keep the
    existing full decode fallback for unknown feature versions.
- Risk / tradeoffs:
  - Feature layout/version assumptions must stay explicit so older blobs still fall back
    safely.
- Visual impact: None
- Validation plan:
  - Add metric-decoder parity tests against the existing full decode path.
  - Extend duplicate-filter similarity tests.
  - Rerun `cargo test -p sempal --lib app::controller::library::wavs::similar`.

### [x] 6. Narrow vendor browser-row cache invalidation and retain per-row geometry
- Classification: Architecture improvement
- Confidence: Medium
- ROI: Medium
- Effort: L
- Expected impact: frame time, CPU, p95 interaction latency on real scene rebuilds
- Completed: 2026-04-17 (vendor `eda74e21`, `perf(browser-rows): retain geometry across selection changes`)
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/cache.rs:80-118` keys browser-row caching
    at window scope and clears the retained truncation cache whenever the
    whole-window truncation key changes.
  - `vendor/radiant/src/gui/native_shell/state/browser_rows/windowing/projection.rs:137-224`
    recomputes text layout, truncation, inline-tag geometry, and owned strings for every
    row in the visible window on cache misses.
  - `vendor/radiant/src/gui/native_shell/state/browser_rows/windowing/projection.rs:17-32`
    and `vendor/radiant/src/gui/native_shell/state/browser_rows/truncation.rs:103-120`
    still pay whole-slice scans/hashing to build cache keys.
  - Fresh retained-bridge probe numbers are already low (`browser_rows_window.p95_us = 11`),
    which is why this sits below the controller and measurement work.
- Recommended change:
  - Separate stable per-row text/geometry caches from volatile selection/focus overlay state.
  - Key cached row geometry by row identity plus width bucket instead of whole-window state
    where possible.
  - Replace repeated visible-window scans for focused/selected hints with retained positions.
- Risk / tradeoffs:
  - Incorrect row identity or truncation invalidation will surface as stale labels/rects.
- Visual impact: None
- Validation plan:
  - Extend vendor browser-row rendering, truncation, and virtualization tests.
  - Manually inspect dense browser rows and focus transitions at multiple viewport sizes.
  - Rerun `cargo nextest run --manifest-path vendor/radiant/Cargo.toml`.

### [ ] 7. After measurement parity, trial progressive startup reveal instead of hidden eager full-scene launch
- Classification: Architecture improvement
- Confidence: Medium
- ROI: Medium
- Effort: M
- Expected impact: startup, CPU, perceived responsiveness
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_startup/policy.rs:4-13`
    launches hidden and disables deferred first-model pull by default.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_startup/initialization.rs:191-199`
    marks layout/model dirty and rebuilds the first scene before reveal.
  - `docs/performance_qa.md` expects startup-first-paint tracking, but the Windows guard
    still cannot collect it today.
- Recommended change:
  - Once item 3 lands, compare the current eager hidden path against a conservative
    placeholder-first or deferred-heavy refresh startup path.
  - Keep the default policy only if first-present latency fails to improve without
    visible pop-in.
- Risk / tradeoffs:
  - Earlier reveal can expose partially populated UI, motion hitching, or obvious pop-in.
- Visual impact: Needs review
- Validation plan:
  - Run repeated startup captures on Windows before and after.
  - Manually review sandbox launches for pop-in, text stability, and first-interaction feel.
  - Rerun `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.

## Open Questions / Missing Definitions

- None blocking for Phase 2 ordering today.
- Startup policy experiments in item 7 should still be manually signed off for visual quality
  before they remain enabled by default.

## Rejected Ideas

- `[-]` Treat vendor browser-row cache work as the top priority.
  - Why it was considered: the vendor row-window code still performs full-window rebuild work.
  - Why it was rejected: fresh retained-runtime probe numbers show the bridge path is already
    cheap (`retained_app_model_projection_p95_us = 15`, `browser_rows_window.p95_us = 11`),
    so controller prep and sync pipeline work have higher immediate ROI.
  - Missing evidence: no fresh live artifact yet shows vendor row-window cache misses as the
    current top shipped-runtime bottleneck.

- `[-]` Spend the next cycle on waveform-motion-path optimization.
  - Why it was considered: waveform paths are usually a common UI hotspot.
  - Why it was rejected: fresh guard numbers are already low
    (`waveform_interaction_latency.p95_us = 208`, `idle_cursor_motion_latency.p95_us = 34`).
  - Missing evidence: no current measurement shows waveform motion dominating p95 latency.
