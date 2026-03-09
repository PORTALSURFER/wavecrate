# Cleanup Plan (ROI Ranked)

Generated: 2026-03-09 (UTC)
Phase: 1 complete, awaiting explicit Phase 2 confirmation
Status legend: `[ ]` pending, `[x]` done
Project language/tooling: Rust 2024 Cargo workspace (`sempal` + `apps/*` + `tools/*` + `vendor/radiant`)
Canonical local CI command: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Ordered Backlog

- [x] 1) Decompose the retained native Vello runtime into focused runtime, redraw, input, text, and startup modules
  - ROI/Effort: High / L
  - Why it matters: `vendor/radiant` runtime code is the largest single cleanup hotspot in the repo; one file currently owns event-loop state, redraw scheduling, cache fingerprints, immediate input paths, text editing, startup reveal policy, and application handler wiring.
  - Evidence:
    - `vendor/radiant/src/gui_runtime/native_vello.rs` is 4362 LOC.
    - The file mixes initialization (`initialize_runtime` around `:1551`), redraw/build flow (`rebuild_scene_if_needed` around `:1818`, `redraw` around `:2956`), immediate input/drag paths (`:2119-2363`), text-editing state (`:3229-3919`), and `ApplicationHandler` methods (`:4024-4478`).
    - The paired test surface is also oversized: `vendor/radiant/src/gui_runtime/native_vello/tests.rs` is 2121 LOC.
  - Recommended change: Split `native_vello.rs` into focused modules such as `runtime_state`, `startup`, `redraw_pipeline`, `immediate_input`, `text_input`, and `profiling`, then split the test file to match those seams.
  - Risk/tradeoffs: High. This is performance-sensitive event-loop code, so extraction mistakes can regress input latency or redraw ordering.
  - Suggested validation: `cargo nextest run --manifest-path vendor/radiant/Cargo.toml native_vello`, snapshot tests under `vendor/radiant`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
  - Completed: 2026-03-09 — `vendor/radiant` commit `5d4e3aa5`

- [ ] 2) Split native-shell interaction state and frame building by responsibility
  - ROI/Effort: High / L
  - Why it matters: native-shell state, hit testing, static-frame building, waveform overlay geometry, and render-time caches are spread across a small cluster of giant files, which raises review cost and makes behavior changes hard to localize.
  - Evidence:
    - `vendor/radiant/src/gui/native_shell/state.rs` is 3645 LOC.
    - `vendor/radiant/src/gui/native_shell/state/frame_build.rs` is 2288 LOC, with one dominant builder entrypoint starting around `build_frame_with_style_into_with_motion_sinks` (`:18`).
    - `vendor/radiant/src/gui/native_shell/state/waveform_segments.rs` is 1014 LOC and owns segment routing plus waveform overlay emission.
    - `vendor/radiant/src/gui/native_shell/state/tests.rs` is 3651 LOC.
  - Recommended change: Separate state/cache ownership, hit-testing, browser-row caching, static-frame composition, and waveform overlay rendering into smaller modules, and split the tests to sit beside the extracted behavior.
  - Risk/tradeoffs: High. The payoff is large, but any extraction here can disturb hit-testing, animation, or retained-scene invalidation behavior.
  - Suggested validation: `cargo nextest run --manifest-path vendor/radiant/Cargo.toml`, targeted snapshot suites, then `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.

- [ ] 3) Split `src/app/controller/playback/transport.rs` by interaction domain
  - ROI/Effort: High / M
  - Why it matters: the main playback transport file is the densest controller SRP violation left in `src/`; it mixes unrelated waveform, loop, seek, playback-start, volume, and escape behavior in one place.
  - Evidence:
    - `src/app/controller/playback/transport.rs` is 878 LOC.
    - Selection/edit drag logic spans roughly `:19-184`.
    - Loop-toggle state and loop playback policy span roughly `:192-394`.
    - Seek/debounce behavior spans roughly `:395-456`.
    - BPM snapping spans roughly `:457-592`.
    - Playback start helpers span roughly `:593-670`.
    - Volume persistence and escape handling span roughly `:671-763`.
  - Recommended change: Extract focused modules such as `selection_drag`, `looping`, `seek`, `playback_start`, and `volume`, keeping the external controller surface stable.
  - Risk/tradeoffs: Medium. Behavior should stay stable, but playback and selection edge cases need to stay covered during the split.
  - Suggested validation: transport in-file tests, `src/app/controller/tests/playback_loop.rs`, `src/app/controller/tests/volume.rs`, `src/app/controller/tests/waveform_nav_cursor.rs`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 4) Decompose the native bridge reducer and retained projection-cache surface into smaller modules
  - ROI/Effort: High / M
  - Why it matters: the retained projection bridge is now a core runtime subsystem, but action coalescing, invalidation policy, projection scheduling, and projection-key/materialization logic remain tightly coupled.
  - Evidence:
    - `src/app_core/native_bridge.rs` is 777 LOC.
    - It mixes pending waveform action batching (`PendingWaveformActions` around `:100-283`), preparation/projection scheduling (`:308-380`), immediate action reduction (`:401-677`), and bridge trait wiring (`:698-803`).
    - `src/app_core/native_bridge/projection_cache.rs` keeps a wide cache-key schema starting around `:74`, while the `projection_cache/` submodules still repeat per-segment materialization concerns.
    - The test surface is already large: `src/app_core/native_bridge/tests.rs` is 1142 LOC.
  - Recommended change: Split bridge reduction into explicit reducer/coalescer/invalidation/projection modules, and further narrow the projection-cache key/materialization helpers so segment refresh logic changes do not require touching multiple coupled files.
  - Risk/tradeoffs: Medium-high. This code is hot-path infrastructure, so module moves need to preserve projection-key and invalidation semantics exactly.
  - Suggested validation: targeted `app_core::native_bridge` tests, `cargo test --doc`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.

- [ ] 5) Further split native-shell projection, especially browser projection and retained row-window refresh
  - ROI/Effort: High / M
  - Why it matters: staged projection has improved, but browser projection still mixes row-windowing, cache refresh, preload decisions, and row materialization in one file, which keeps app-core/native-shell coupling higher than necessary.
  - Evidence:
    - `src/app_core/native_shell/browser_projection.rs` is 515 LOC.
    - It mixes projection inputs (`:24-44`), frame metadata projection (`:47-81`), row-window materialization (`:84-186`), BPM preload logic (`:188-313`), and browser chrome projection (`:315-497`).
    - `src/app_core/native_shell.rs` already treats projection as staged work around `:159`, `:175`, and `:191`, but the browser path still holds too many concerns in one module.
  - Recommended change: Split browser projection into focused `frame`, `rows`, `preload`, and `chrome` modules, with helper tests close to the extracted row-window and preload logic.
  - Risk/tradeoffs: Medium. The split is behavior-preserving if row ordering and preload windows remain identical, but browser regressions would be user-visible.
  - Suggested validation: targeted `app_core::native_shell` tests, controller browser integration tests, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 6) Deduplicate fuzzy-search cache and scoring logic shared by synchronous and worker browser search paths
  - ROI/Effort: High / M
  - Why it matters: the current search stack duplicates cache reuse and fuzzy-score computation across two implementations, which raises correctness risk whenever scoring policy changes.
  - Evidence:
    - `src/app/controller/library/wavs/browser_search.rs` defines `QueryScoreCacheEntry`, bounded query-score caching, exact-hit promotion, prefix-cache reuse, and fuzzy scoring around `:15-59` and `:87-184`.
    - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` duplicates the same concepts for the worker path around `:108-221`.
  - Recommended change: Move shared query-cache and score-resolution primitives into one common module used by both immediate and worker search paths.
  - Risk/tradeoffs: Medium. Search order and cache-hit behavior must remain stable across both paths.
  - Suggested validation: `src/app/controller/tests/browser_core.rs`, new shared unit tests for exact-hit promotion and prefix reuse, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 7) Split `src/sample_sources/db/file_ops_journal.rs` into journal codec, persistence API, and reconciliation executor modules
  - ROI/Effort: High / M
  - Why it matters: this is crash-recovery code, so isolation and readability are higher-value than raw LOC; recovery semantics are still harder to reason about than they should be.
  - Evidence:
    - `src/sample_sources/db/file_ops_journal.rs` is 592 LOC even after the recent coverage/doc pass.
    - The file still mixes entry types, SQL decoding, malformed-row handling, and reconciliation/finalize logic from `list_entries` (`:301`) through `reconcile_source_entry` (`:573`).
  - Recommended change: Split the file into `entry`, `codec`, `store`, and `reconcile` modules while keeping the stage contract documented in `docs/file_ops_journal_recovery.md`.
  - Risk/tradeoffs: Medium. Any split here must preserve startup recovery behavior and the current test matrix.
  - Suggested validation: existing journal tests in `src/sample_sources/db/file_ops_journal/tests.rs` plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 8) Break waveform loading into load-request, finalize, and duration-metadata units and retire the legacy wide-argument shim
  - ROI/Effort: High / M
  - Why it matters: `waveform_loading.rs` still mixes cache lookups, decode/stretch work, UI apply logic, deferred metadata writeback, and a compatibility shim that keeps a too-wide signature alive.
  - Evidence:
    - `src/app/controller/library/wavs/waveform_loading.rs` is 630 LOC.
    - `finish_waveform_load(...)` around `:175-194` still uses `#[allow(clippy::too_many_arguments)]`.
    - Deferred metadata persistence is a separate dense block around `:374-499`.
  - Recommended change: Split the file into `load_request`, `load_finalize`, and `duration_metadata` helpers, and migrate callers away from the legacy compatibility shim.
  - Risk/tradeoffs: Medium. The split touches cache-hit and same-path refresh behavior, which must remain stable.
  - Suggested validation: existing waveform load tests, `src/app/controller/tests/waveform_cache_loading.rs`, broader waveform controller tests, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 9) Decompose destructive selection-edit workflows into focused application services
  - ROI/Effort: High / M
  - Why it matters: selection-edit code still bundles preview math, file writes, DB updates, cache refreshes, undo capture, playback carry-over, and UI/status mutations in one module.
  - Evidence:
    - `src/app/controller/library/selection_edits/mod.rs` is 507 LOC.
    - `crop_waveform_selection_to_new_sample` spans roughly `:148-248` and mixes buffer mutation, fade handling, file output, DB updates, cache/index refresh, undo capture, playback continuation, and status reporting.
  - Recommended change: Extract a focused “write edited clip” service and keep controller/UI follow-up behavior in thinner orchestration methods.
  - Risk/tradeoffs: Medium. This path mutates both files and DB state, so test coverage has to expand before moving code aggressively.
  - Suggested validation: extend `src/app/controller/tests/waveform.rs` for crop-to-new-sample and playback-continuation branches, then run `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 10) Remove duplicated `wav_files` row mapping and upsert SQL in the source DB layer
  - ROI/Effort: High / M
  - Why it matters: persistence fixes currently require editing several near-identical SQL and row-mapping blocks, which increases regression risk for DB schema work.
  - Evidence:
    - `src/sample_sources/db/read.rs` repeats `WavEntry` row decoding in at least three places (`:31`, `:70`, `:230`).
    - `src/sample_sources/db/write.rs` repeats near-identical `INSERT INTO wav_files ... ON CONFLICT` statements at `:25`, `:162`, `:198`, `:236`, and `:278`.
  - Recommended change: Centralize `wav_files` row decoding and introduce a shared upsert builder/helper for the write paths.
  - Risk/tradeoffs: Medium. SQL refactors can accidentally change null/default handling if the shared helper is too clever.
  - Suggested validation: `tests/unit/source_db_mod_tests.rs`, DB-layer unit tests under `src/sample_sources/db`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 11) Isolate clipboard import into an explicit workflow/state machine and add failure-path coverage
  - ROI/Effort: Medium-High / M
  - Why it matters: clipboard import performs real filesystem mutations and journal bookkeeping, but most failure branches still live inside one wide function with limited focused coverage.
  - Evidence:
    - `src/app/controller/ui/clipboard_paste.rs` is 475 LOC.
    - `run_clipboard_paste_job` spans roughly `:239-449` and owns validation, duplicate naming, staging, copy, DB writes, finalize rename, cleanup, progress updates, and error accumulation.
    - Existing tests are light compared with the mutation surface.
  - Recommended change: Extract a staged workflow runner with explicit stage/result types so failure-path behavior can be unit-tested without full controller setup.
  - Risk/tradeoffs: Medium. This touches journal-backed file operations, so the split should prioritize observability over clever abstractions.
  - Suggested validation: existing clipboard/external drop tests, new failure-path unit tests, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 12) Separate map UI commands from UMAP query/repository code
  - ROI/Effort: Medium / M
  - Why it matters: map controller intent and raw SQL/query code are coupled in one file, which makes the map path harder to test and obscures the intended ownership boundary.
  - Evidence:
    - `src/app/controller/ui/map_view.rs` is 460 LOC.
    - UI/controller commands are at the top (`set_browser_tab`, `focus_map_sample_and_preview`).
    - Connection caching and raw loaders live lower in the same file: `open_source_db` (`:205`), `load_umap_bounds` (`:247`), `load_umap_points` (`:304`), `load_umap_point_for_sample` (`:385`), and `load_umap_cluster_centroids` (`:407`).
  - Recommended change: Move DB/query functions into a dedicated map repository module and keep `map_view.rs` as controller-facing orchestration only.
  - Risk/tradeoffs: Medium. Query extraction is straightforward, but test fixtures will need to move with the new repository boundary.
  - Suggested validation: `src/app/controller/tests/map_view.rs`, new query-layer tests, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 13) Split the updater-helper and installer UI bridges into reducer, background-task, and model-projection modules
  - ROI/Effort: Medium / M
  - Why it matters: both workspace app UIs are effectively small state machines, but their async polling, reducer logic, and model composition are still concentrated in single files with very light direct coverage.
  - Evidence:
    - `apps/updater-helper/src/ui.rs` is 530 LOC; `UpdateNativeBridge` owns release fetching, apply orchestration, log buffering, selection state, app model construction, and action reduction. The file only has two tests starting around `:517`.
    - `apps/installer/src/ui.rs` is 461 LOC; `InstallerNativeBridge` owns wizard navigation, worker polling, finish actions, icon loading, and model rendering. The file only has one direct test starting around `:481`.
  - Recommended change: Split each UI into `state`, `background`, and `view_model` modules, then add reducer-style tests for success, failure, retry, and tab/step transitions.
  - Risk/tradeoffs: Medium. These are smaller apps, but mistakes here show up directly in install/update flows.
  - Suggested validation: app-local unit tests for both workspace apps, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 14) Prune or explicitly justify remaining `#[allow(dead_code)]` suppressions in controller and integration code
  - ROI/Effort: Medium / S
  - Why it matters: the remaining suppressions blur the line between intentional compatibility seams and abandoned API surface, which weakens future cleanup signals.
  - Evidence:
    - Controller-area suppressions remain in `src/app/controller/library/source_folders/selection/filter.rs:34`, `src/app/controller/library/source_folders/selection/ops.rs:232`, `src/app/controller/state/cache.rs:86`, `src/app/controller/library/analysis_jobs/db/artifacts.rs:12`, and `src/app/controller/jobs/messages.rs:162`/`:171`.
    - Persistence/integration suppressions also remain in `src/sample_sources/scanner/scan/runner.rs:68`, `src/updater/github.rs:29`, and `apps/installer/src/install.rs:17`/`:21`/`:23`.
  - Recommended change: remove unused helpers/fields where possible, move test-only helpers behind `#[cfg(test)]`, and add short rationale comments only for truly intentional dormant compatibility seams.
  - Risk/tradeoffs: Low-medium. The change is small, but some suppressed fields may still be serving serialization or compatibility roles that need to stay explicit.
  - Suggested validation: `cargo clippy --workspace --all-targets` and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

## Progress Log

- 2026-03-09: Phase 1 cleanup audit refreshed from the current workspace state; no Phase 2 implementation has started.
- 2026-03-09: Read repository guidance first (`AGENTS.md`, `README.md`, `docs/README.md`, active plans, and `MEMORY.md`) before the audit.
- 2026-03-09: Detected a Rust 2024 Cargo workspace with `vendor/radiant` as the primary runtime/UI hotspot and `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1` as the canonical local CI parity command on this machine.
