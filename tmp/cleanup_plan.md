# Cleanup Plan (ROI Ranked)

Generated: 2026-03-09 (UTC)
Phase: 1 complete, awaiting explicit Phase 2 confirmation
Status legend: `[ ]` pending, `[x]` done
Project language/tooling: Rust 2024 Cargo workspace (`sempal` + `apps/*` + `tools/*` + `vendor/radiant`)
Canonical local CI command: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Ordered Backlog

- [ ] 1) Split `vendor/radiant` native-shell state and frame building by responsibility
  - ROI/Effort: High / L
  - Why it matters: the retained native-shell state surface still concentrates cached row models, sync-from-model logic, static frame composition, motion overlay routing, and test scaffolding across a few very large files, which keeps changes risky and hard to review.
  - Evidence:
    - `vendor/radiant/src/gui/native_shell/state.rs` is 3864 LOC.
    - `vendor/radiant/src/gui/native_shell/state/frame_build.rs` is 2311 LOC and centers the main static-frame builder at `build_frame_with_style_into_with_motion_sinks` (`:18`).
    - `vendor/radiant/src/gui/native_shell/state/tests.rs` is 3940 LOC.
    - `vendor/radiant/src/gui/native_shell/state/waveform_segments.rs` is 1057 LOC.
    - `vendor/radiant/src/gui/native_shell/state/browser_rows.rs` is 702 LOC.
  - Recommended change: split state/cache ownership, model-sync, frame-build, overlay, and browser-row concerns into focused sibling modules, then split tests to match those seams.
  - Risk/tradeoffs: High. This is central retained-scene code, so module moves can regress hit testing, redraw invalidation, or snapshot behavior if not done carefully.
  - Suggested validation: targeted `radiant` native-shell tests and snapshot fixtures, then `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.

- [ ] 2) Finish decomposing `vendor/radiant` native Vello runtime and its test surface
  - ROI/Effort: High / L
  - Why it matters: the runtime split has started, but immediate input handling, static-scene fingerprinting, event-loop orchestration, and the oversized test surface are still too coupled for safe incremental changes.
  - Evidence:
    - `vendor/radiant/src/gui_runtime/native_vello.rs` is 2612 LOC.
    - `vendor/radiant/src/gui_runtime/native_vello/input.rs` is 1114 LOC.
    - `vendor/radiant/src/gui_runtime/native_vello/tests.rs` is 2328 LOC.
    - `vendor/radiant/src/gui_runtime/native_vello.rs` still owns fingerprinting helpers (`:178-581`), frame-state coordination (`:599-689`), and the main runner (`NativeVelloRunner` at `:690`).
  - Recommended change: continue the split into explicit runtime state, scene fingerprinting, pointer/input routing, redraw scheduling, and test modules with smaller behavior-specific fixtures.
  - Risk/tradeoffs: High. This is hot-path event-loop code; changes can easily impact responsiveness or redraw correctness.
  - Suggested validation: targeted `radiant` runtime tests, snapshot suites, perf-sensitive interaction checks, then `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.

- [ ] 3) Split `vendor/radiant/src/app/mod.rs` into panel-model, action, and bridge-boundary modules
  - ROI/Effort: High / M
  - Why it matters: one large public module currently defines most of the UI projection contract, which obscures ownership boundaries and makes the public surface harder to navigate and document.
  - Evidence:
    - `vendor/radiant/src/app/mod.rs` is 1523 LOC.
    - It mixes browser/source/waveform/update panel models, `UiAction`, dirty-segment bookkeeping, motion projection, and the `NativeAppBridge` trait in one file.
    - Major section boundaries include `AppModel` (`:681`), `UiAction` (`:777`), `DirtySegments` (`:1193`), `NativeMotionModel` (`:1326`), and bridge hooks (`:1439-1520`).
  - Recommended change: split the public app surface into focused modules such as `models/browser`, `models/waveform`, `actions`, `dirty_segments`, and `bridge`.
  - Risk/tradeoffs: Medium-high. The API is widely consumed, so the main risk is churn in imports and docs rather than behavior changes.
  - Suggested validation: `radiant` unit tests, `cargo doc -p sempal --no-deps`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.

- [ ] 4) Decompose `src/app_core/native_bridge.rs` and related metrics/tests into reducer, invalidation, projection, and metrics modules
  - ROI/Effort: High / M
  - Why it matters: the retained bridge is now a core runtime subsystem, but action batching, invalidation, projection scheduling, and telemetry still change together more often than they should.
  - Evidence:
    - `src/app_core/native_bridge.rs` is 827 LOC.
    - `src/app_core/native_bridge/metrics.rs` is 901 LOC.
    - `src/app_core/native_bridge/tests.rs` is 1253 LOC.
    - `PendingWaveformActions` starts at `src/app_core/native_bridge.rs:110`, with batching/application logic around `:477-489`, and the `NativeAppBridge` implementation starts at `:696`.
  - Recommended change: split the bridge into reducer/coalescer, invalidation policy, projection handoff, and metrics modules; split tests to sit beside those seams.
  - Risk/tradeoffs: Medium-high. Hot-path behavior must remain identical, especially for coalesced waveform actions and projection invalidation.
  - Suggested validation: targeted native-bridge tests, browser/projection integration tests, then `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.

- [ ] 5) Further split app-core browser projection into focused row-window, chrome, and preload modules
  - ROI/Effort: High / M
  - Why it matters: browser projection is now one of the last concentrated app-core/native-shell coupling points, and row-windowing plus preload policy still live in the same file.
  - Evidence:
    - `src/app_core/native_shell/browser_projection.rs` is 540 LOC.
    - It mixes row-projection inputs (`:31`), row-model projection (`:95-316`), browser-panel assembly (`:317-453`), and browser chrome projection (`:454-540`).
  - Recommended change: split into `rows`, `panel`, `chrome`, and `preload` helpers with tests close to the extracted row-window and preload logic.
  - Risk/tradeoffs: Medium. Browser row ordering and preload behavior must stay stable.
  - Suggested validation: targeted `app_core::native_shell` tests, browser integration tests, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 6) Split `src/app/controller/playback/transport.rs` by interaction domain
  - ROI/Effort: High / M
  - Why it matters: this remains the densest controller SRP violation in `src/`; selection dragging, loop toggling, seeking, BPM snapping, playback start, and volume behavior still live together.
  - Evidence:
    - `src/app/controller/playback/transport.rs` is 965 LOC.
    - Selection drag entrypoints occupy `:19-184`.
    - Loop toggle helpers span `:192-394`.
    - Seek and queued-commit behavior span `:395-456`.
    - BPM snapping spans `:457-592`.
    - Playback start and replay helpers begin at `:593`.
  - Recommended change: extract focused modules such as `selection_drag`, `looping`, `seek`, `bpm_snap`, `playback_start`, and `volume`.
  - Risk/tradeoffs: Medium. Playback and selection edge cases are user-visible, so coverage has to stay intact during the split.
  - Suggested validation: transport-focused tests, waveform navigation/loop/volume controller tests, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 7) Deduplicate fuzzy-search scoring and cache reuse shared by synchronous and worker browser search paths
  - ROI/Effort: High / M
  - Why it matters: scoring policy changes currently need to be kept in sync across two implementations, which is an avoidable correctness hazard.
  - Evidence:
    - `src/app/controller/library/wavs/browser_search.rs` defines `QueryScoreCacheEntry` and synchronous fuzzy-score cache reuse starting at `:17`, with the main scoring path around `:87-184`.
    - `src/app/controller/library/wavs/browser_search_worker/cache.rs` defines `WorkerQueryScoreCacheEntry`.
    - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` duplicates exact-hit and prefix-cache reuse logic around `:108-221`.
  - Recommended change: move shared cache/scoring primitives into one common module used by both synchronous and worker search paths.
  - Risk/tradeoffs: Medium. Search ranking and cache-hit semantics must remain stable.
  - Suggested validation: browser search unit tests in both paths, plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 8) Split `src/sample_sources/db/file_ops_journal.rs` into entry, store, and reconciliation modules
  - ROI/Effort: High / M
  - Why it matters: this is crash-recovery code, so improving isolation and reviewability has outsized value even without changing behavior.
  - Evidence:
    - `src/sample_sources/db/file_ops_journal.rs` is 628 LOC.
    - CRUD helpers are mixed with recovery logic: `remove_entry` (`:293`), `list_entries` (`:301`), reconciliation loop (`:458-572`), and `reconcile_source_entry` (`:573`).
    - Tests are already separated into `src/sample_sources/db/file_ops_journal/tests.rs`, which makes a production split easier.
  - Recommended change: separate entry/schema types, persistence API, and reconciliation executor logic into focused modules.
  - Risk/tradeoffs: Medium. Startup recovery semantics must stay exact.
  - Suggested validation: existing journal tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 9) Break waveform loading into request, finalize, and metadata-persistence units and retire the compatibility shim
  - ROI/Effort: High / M
  - Why it matters: waveform loading still bundles cache lookups, decode/stretch work, UI apply behavior, and deferred metadata persistence behind one wide surface.
  - Evidence:
    - `src/app/controller/library/wavs/waveform_loading.rs` is 668 LOC.
    - The shared/owned finalize parameter structs start at `:15` and `:26`.
    - `finish_waveform_load_shared` starts at `:125`.
    - `finish_waveform_load_owned` starts at `:164`.
    - `finish_waveform_load` remains as a compatibility wrapper at `:190`.
  - Recommended change: extract request/finalize/metadata modules and migrate remaining callers off the compatibility wrapper.
  - Risk/tradeoffs: Medium. Cache-hit and same-path refresh behavior must remain stable.
  - Suggested validation: waveform-loading tests, cache-loading controller tests, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 10) Decompose destructive selection-edit workflows into focused write services and thin controller orchestration
  - ROI/Effort: High / M
  - Why it matters: destructive selection edits still mix buffer transforms, file writes, DB updates, undo capture, cache refreshes, and playback/UI follow-up in one module.
  - Evidence:
    - `src/app/controller/library/selection_edits/mod.rs` is 551 LOC.
    - `crop_waveform_selection_to_new_sample` starts at `:148`.
    - `normalize_waveform_selection` starts at `:280`.
    - `reverse_waveform_selection` starts at `:337`.
  - Recommended change: extract explicit edit-application services and keep controller methods as smaller orchestration shells.
  - Risk/tradeoffs: Medium. These paths touch both files and persisted state, so regression risk is real without focused tests.
  - Suggested validation: waveform editing controller tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 11) Remove duplicated `wav_files` row mapping and upsert SQL in the source DB layer
  - ROI/Effort: High / M
  - Why it matters: persistence fixes still require synchronized edits across several near-identical SQL statements and row-decoding blocks.
  - Evidence:
    - `src/sample_sources/db/read.rs` repeats `WavEntry` decoding at least three times (`:31`, `:70`, `:230`).
    - `src/sample_sources/db/write.rs` repeats near-identical `INSERT INTO wav_files ... ON CONFLICT` statements at `:25`, `:162`, `:198`, `:236`, and `:278`.
  - Recommended change: centralize row decoding and introduce shared helpers for the main `wav_files` upsert variants.
  - Risk/tradeoffs: Medium. SQL refactors can accidentally shift null/default handling if the shared helper is over-generalized.
  - Suggested validation: DB-layer unit tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 12) Split `src/audio/output.rs` into device discovery, stream setup, and callback runtime modules
  - ROI/Effort: Medium-High / M
  - Why it matters: audio device enumeration, stream configuration, callback mixing, and monitor-sink helpers are still coupled in one file, which makes platform-specific changes harder to isolate.
  - Evidence:
    - `src/audio/output.rs` is 696 LOC.
    - `CpalAudioStream` starts at `:188`.
    - Host/device discovery lives at `available_hosts` (`:339`) and `available_devices` (`:357`).
    - Stream opening starts at `open_output_stream` (`:412`).
    - Real-time callback mixing starts at `process_audio_callback` (`:594`).
  - Recommended change: split discovery/config resolution, stream construction, and callback/runtime state into focused modules.
  - Risk/tradeoffs: Medium. Audio code is platform-sensitive and callback safety must remain straightforward.
  - Suggested validation: targeted audio-output tests, platform smoke checks where available, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 13) Isolate clipboard paste/import into an explicit staged workflow and add failure-path coverage
  - ROI/Effort: Medium-High / M
  - Why it matters: clipboard import performs real filesystem mutations and journal-backed bookkeeping, but most failure behavior still lives inside one wide function.
  - Evidence:
    - `src/app/controller/ui/clipboard_paste.rs` is 475 LOC.
    - `run_clipboard_paste_job` starts at `:239` and owns validation, naming, staging, copy, DB writes, finalize rename, cleanup, progress updates, and summary building.
    - `begin_clipboard_paste_job` starts at `:168`, which means the background job lifecycle and workflow logic are tightly coupled.
  - Recommended change: model the paste flow as explicit stages/results so failure branches can be unit-tested without full controller orchestration.
  - Risk/tradeoffs: Medium. This path mutates both files and journal state, so refactors must preserve rollback behavior.
  - Suggested validation: clipboard/external-drop tests, focused failure-path tests, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 14) Separate map UI orchestration from UMAP query/repository code
  - ROI/Effort: Medium / M
  - Why it matters: controller-facing map commands and raw SQL/query helpers still live together, which blurs the intended boundary and limits focused testing.
  - Evidence:
    - `src/app/controller/ui/map_view.rs` is 460 LOC.
    - UI/controller entrypoints start at `focus_map_sample_and_preview` (`:61`).
    - DB/query helpers live in the same file: `open_source_db` (`:205`), `open_source_db_for_id` (`:237`), `load_umap_bounds` (`:247`), `load_umap_points` (`:304`), and `load_umap_cluster_centroids` (`:407`).
  - Recommended change: move the query/database helpers into a dedicated repository module and keep `map_view.rs` controller-facing.
  - Risk/tradeoffs: Medium. Query extraction is straightforward, but fixtures and connection ownership need to stay clear.
  - Suggested validation: map-view tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 15) Split installer/updater helper UIs into reducer, background-task, and view-model modules
  - ROI/Effort: Medium / M
  - Why it matters: both workspace app UIs are small state machines, but async work, reducer logic, and model projection still live in single files that are larger than they need to be.
  - Evidence:
    - `apps/updater-helper/src/ui.rs` is 564 LOC, with `UpdateNativeBridge` at `:64`, action reduction at `:444`, and tests at `:517`.
    - `apps/installer/src/ui.rs` is 461 LOC, with `InstallerNativeBridge` at `:48`, action reduction at `:365`, and tests at `:482`.
  - Recommended change: split each app UI into `state`, `background`, and `view_model` modules and expand reducer-style tests around failure and retry paths.
  - Risk/tradeoffs: Medium. These apps are smaller, but mistakes directly affect install/update flows.
  - Suggested validation: app-local tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 16) Prune or explicitly justify remaining `#[allow(dead_code)]` and `#[allow(clippy::too_many_arguments)]` suppressions
  - ROI/Effort: Medium / S
  - Why it matters: the remaining suppressions hide whether code is intentionally dormant, test-only, or simply overdue for cleanup.
  - Evidence:
    - Non-vendor suppressions remain in `src/updater/github.rs:29`, `src/app/controller/library/wavs/waveform_loading.rs:189`, `src/app/controller/ui/loading.rs:114`, `src/app/controller/jobs/messages.rs:162`/`:171`, `src/app/controller/state/cache.rs:86`, `src/app/controller/library/source_folders/selection/filter.rs:34`, `src/app/controller/library/source_folders/selection/ops.rs:232`, `src/app/controller/library/analysis_jobs/db/artifacts.rs:12`, `src/sample_sources/scanner/scan/runner.rs:68`, and `apps/installer/src/install.rs:17`/`:21`/`:23`.
    - Vendor suppressions still exist too, but Phase 2 should start with the non-vendor ones because they are lower-risk and directly owned here.
  - Recommended change: remove truly unused helpers/fields, move test-only helpers behind `#[cfg(test)]`, and leave short rationale comments only where compatibility requires the suppression.
  - Risk/tradeoffs: Low-medium. Some suppressed fields may be serving serialization or compatibility roles that need to stay explicit.
  - Suggested validation: `cargo clippy --workspace --all-targets` and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

## Progress Log

- 2026-03-09: Phase 1 cleanup audit refreshed from the current post-merge workspace state; no Phase 2 implementation has started.
- 2026-03-09: Read repository guidance first (`AGENTS.md`, `README.md`, `docs/README.md`, active plans, and `MEMORY.md`) before the audit.
- 2026-03-09: Confirmed the canonical local CI parity command on this Windows machine is `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
