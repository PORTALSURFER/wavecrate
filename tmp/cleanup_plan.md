# Cleanup Plan (ROI Ranked)

Generated: 2026-03-09 (UTC)
Phase: 2 in progress
Status legend: `[ ]` pending, `[x]` done
Project language/tooling: Rust 2024 Cargo workspace (`sempal` + `apps/*` + `tools/*` + `vendor/radiant`)
Canonical local CI command: `bash scripts/ci_local.sh`

## Audit Notes

- Public API docs are already enforced at the workspace level via `missing_docs = "deny"` in the root and `vendor/radiant` manifests.
- The highest cleanup ROI is concentrated in duplication, mixed-responsibility controller/persistence flows, and a small set of oversized runtime modules.
- Phase 2 should start with locally owned, behavior-preserving extractions before the large `vendor/radiant` splits.

## Ordered Backlog

- [x] 1) Fix public `analysis` contract drift and add focused behavior tests
  - ROI/Effort: High / M
  - Why it matters: several public analysis APIs currently advertise behavior that the implementation does not provide, which is higher-risk than ordinary structural debt because downstream callers can make the wrong assumptions.
  - Evidence:
    - `src/analysis/ann_index/mod.rs` documents `upsert_embedding` (`:88`) and `upsert_embeddings_batch` (`:99`) as insert-or-update APIs, but `src/analysis/ann_index/update.rs` returns early for existing IDs at `:13` and `:39`.
    - `src/analysis/ann_index/mod.rs:118` documents `find_similar` like a read call, but the implementation backfills missing ANN entries by calling `update::upsert_embedding` at `:146`.
    - `src/analysis/mod.rs:13` and `src/analysis/umap.rs:42` expose a UMAP-named surface, while `src/analysis/umap.rs` actually runs `compute_tsne` and emits t-SNE-specific errors at `:53`, `:141`, and `:207`.
    - `src/analysis/mod.rs:80` exposes `infer_embedding`, but the implementation immediately returns a deprecation/removal error at `:86`.
  - Recommended change: decide the true public contract for duplicate ANN IDs and ANN backfill-on-read, then align docs/tests with that contract; clarify the current `umap` naming vs t-SNE-backed implementation; and add direct tests for duplicate-ID behavior, backfill-on-read, and the remaining analysis helpers with validation/sanitization logic.
  - Risk/tradeoffs: Medium. This work may reveal that some callers depend on the current undocumented behavior, so the safest fix may be docs-plus-tests rather than behavior changes in every case.
  - Suggested validation: targeted analysis unit tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.
  - Completion: 2026-03-09 (`16932de4`)

- [x] 2) Remove duplicated `wav_files` row decoding and upsert SQL in the source DB layer
  - ROI/Effort: High / S
  - Why it matters: persistence fixes currently require synchronized edits across repeated row-decoding and near-identical `INSERT ... ON CONFLICT` blocks.
  - Evidence:
    - `src/sample_sources/db/read.rs` is 393 LOC and repeats `query_map` row decoding at `:20`, `:59`, and `:219`.
    - `src/sample_sources/db/write.rs` is 373 LOC and repeats `INSERT INTO wav_files ... ON CONFLICT(path) DO UPDATE` at `:25`, `:162`, `:198`, `:236`, and `:278`.
  - Recommended change: introduce shared wav-row decode helpers and a small set of explicit upsert builders/helpers instead of open-coded SQL variants.
  - Risk/tradeoffs: Low-medium. Shared helpers must not blur the intentional differences between hash/tag/missing variants.
  - Suggested validation: source DB unit tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.
  - Completion: 2026-03-09 (`1fe099ae`)

- [x] 3) Deduplicate fuzzy-search scoring and cache reuse across synchronous and worker browser search paths
  - ROI/Effort: High / M
  - Why it matters: search ranking and cache behavior are implemented twice, so every scoring change carries correctness-drift risk.
  - Evidence:
    - `src/app/controller/library/wavs/browser_search.rs` is 465 LOC and defines `QueryScoreCacheEntry` (`:17`) plus the synchronous cache/scoring path.
    - `src/app/controller/library/wavs/browser_search_worker/cache.rs` defines `WorkerQueryScoreCacheEntry` (`:104`).
    - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` is 467 LOC and duplicates reuse logic in `resolve_query_scores_for_job` (`:98`), `try_reuse_cached_query_scores` (`:170`), and `reusable_prefix_query_scores` (`:186`).
  - Recommended change: extract shared scoring/cache primitives into one module used by both controller and worker paths.
  - Risk/tradeoffs: Medium. Query ranking, exact-hit reuse, and prefix-cache semantics must remain unchanged.
  - Suggested validation: browser search unit tests for sync and worker paths, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.
  - Completion: 2026-03-09 (`0b0be54a`)

- [x] 4) Separate map UI orchestration from UMAP query/repository code
  - ROI/Effort: High / M
  - Why it matters: controller-facing map actions and low-level SQL/query helpers still live together, which blurs ownership and keeps map data access hard to test directly.
  - Evidence:
    - `src/app/controller/ui/map_view.rs` is 483 LOC.
    - Controller entrypoints start near `focus_map_sample_and_preview` (`:54`) and `build_umap_layout` (`:71`).
    - DB/query helpers live in the same file: `open_source_db` (`:205`), `open_source_db_for_id` (`:237`), `load_umap_bounds` (`:247`), `load_umap_points` (`:304`), and `load_umap_cluster_centroids` (`:407`).
  - Recommended change: move DB/query responsibilities into a dedicated repository module and keep `map_view.rs` controller-oriented.
  - Risk/tradeoffs: Medium. Connection ownership and per-source cache lifetime need to stay explicit.
  - Suggested validation: focused map query tests, map-view controller tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.
  - Completion: 2026-03-09 (`f752dec6`)

- [x] 5) Break waveform loading into request, finalize, and metadata-persistence units and retire the compatibility shim
  - ROI/Effort: High / M
  - Why it matters: waveform loading still combines cache lookup, decode/stretch preparation, UI application, and deferred metadata persistence in one wide module.
  - Evidence:
    - `src/app/controller/library/wavs/waveform_loading.rs` is 668 LOC.
    - `FinishWaveformLoadShared` starts at `:15` and `FinishWaveformLoadOwned` at `:26`.
    - `finish_waveform_load_shared` starts at `:125`, `finish_waveform_load_owned` at `:164`, and the compatibility shim `finish_waveform_load` at `:190`.
    - The current tests only cover deferred metadata timing in the tail test module (`:593` onward).
  - Recommended change: split request/preparation, finalize/apply, and deferred-metadata concerns into focused modules, then remove the owned-parameter shim once callers are migrated.
  - Risk/tradeoffs: Medium. Same-path refresh, cache-hit behavior, and loaded-duration persistence timing must remain stable.
  - Suggested validation: waveform-loading tests, cache-loading controller tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.
  - Completion: 2026-03-09 (`8d2c30e8`)

- [ ] 6) Isolate clipboard paste/import into an explicit staged workflow and add real failure-path coverage
  - ROI/Effort: High / M
  - Why it matters: clipboard import performs filesystem mutation plus journal-backed bookkeeping, but most of the workflow still sits in one large background job body.
  - Evidence:
    - `src/app/controller/ui/clipboard_paste.rs` is 491 LOC.
    - `run_clipboard_paste_job` starts at `:239` and owns validation, naming, staging, copy, DB writes, finalize rename, cleanup, progress updates, and summary creation.
    - The current local tests only cover `validate_relative_folder_path` (`:476-491`), not the multi-stage rollback/error paths.
  - Recommended change: model the paste job as explicit stages/results, then add tests for rollback and partial-failure paths without requiring full controller orchestration.
  - Risk/tradeoffs: Medium. Journal/finalize behavior must remain exact or imports could leave stale staged files behind.
  - Suggested validation: clipboard/external-drop tests, staged failure-path tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.

- [ ] 7) Split `src/sample_sources/db/file_ops_journal.rs` into entry, store, and reconciliation modules
  - ROI/Effort: High / M
  - Why it matters: this is crash-recovery code, so clearer boundaries improve reviewability and reduce startup-recovery risk.
  - Evidence:
    - `src/sample_sources/db/file_ops_journal.rs` is 628 LOC.
    - Entry/storage helpers and reconciliation are mixed together: `remove_entry` (`:293`), `list_entries` (`:301`), `reconcile_pending_ops` (`:457`), `reconcile_entry` (`:497`), and `reconcile_source_entry` (`:573`).
    - Tests already live in `src/sample_sources/db/file_ops_journal/tests.rs`, which makes a production split easier.
  - Recommended change: separate journal entry/schema types, persistence API, and reconciliation executor behavior into focused sibling modules.
  - Risk/tradeoffs: Medium. Startup recovery semantics and journal cleanup must remain byte-for-byte compatible in behavior.
  - Suggested validation: journal unit tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.

- [ ] 8) Prune or explicitly justify remaining non-vendor `#[allow(dead_code)]` and `#[allow(clippy::too_many_arguments)]` suppressions
  - ROI/Effort: Medium-High / S
  - Why it matters: the remaining suppressions make it harder to distinguish intentional compatibility shims from simply stale code.
  - Evidence:
    - There are 32 non-test suppressions under `src/`, `apps/`, and `tools/`.
    - Representative examples include `src/app/controller/library/wavs/waveform_loading.rs:189`, `src/app/controller/ui/loading.rs:114`, `src/app/controller/jobs/messages.rs:162`, `src/app/controller/state/cache.rs:86`, `src/sample_sources/scanner/scan/runner.rs:68`, `src/updater/github.rs:29`, and `apps/installer/src/install.rs:17`.
  - Recommended change: remove truly unused code, move test-only helpers behind `#[cfg(test)]`, and add short rationale comments only where compatibility or platform glue requires the suppression.
  - Risk/tradeoffs: Low-medium. A few suppressed fields may still be intentionally retained for serialization/platform integration.
  - Suggested validation: `cargo clippy --workspace --all-targets`, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.

- [ ] 9) Decompose destructive selection-edit workflows into focused write services and thin controller orchestration
  - ROI/Effort: High / M
  - Why it matters: destructive edits still combine sample-buffer transforms, file writes, DB updates, undo capture, cache refreshes, and playback follow-up in one module.
  - Evidence:
    - `src/app/controller/library/selection_edits/mod.rs` is 551 LOC.
    - `crop_waveform_selection_to_new_sample` starts at `:148`.
    - `normalize_waveform_selection` starts at `:280`.
    - `reverse_waveform_selection` starts at `:337`.
  - Recommended change: extract edit-application services and keep controller methods as orchestration shells over those services.
  - Risk/tradeoffs: Medium. These flows touch both files and persisted state, so regression risk is real without focused tests.
  - Suggested validation: waveform-edit controller tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.

- [ ] 10) Split `src/app/controller/playback/transport.rs` by interaction domain
  - ROI/Effort: Medium-High / M
  - Why it matters: selection dragging, loop toggle policy, seeking, BPM snapping, and playback follow-up still live in one dense controller file.
  - Evidence:
    - `src/app/controller/playback/transport.rs` is 965 LOC.
    - Selection drag entrypoints begin at `:19`.
    - Loop toggle logic starts at `:192`.
    - Playback helpers continue through the later half of the file, while tests also live in the same module tail.
  - Recommended change: extract `selection_drag`, `looping`, `seek`, `bpm_snap`, and playback helpers into focused modules with tests aligned to those seams.
  - Risk/tradeoffs: Medium. Playback and selection edge cases are user-visible and need stable coverage during the split.
  - Suggested validation: transport-focused tests, waveform navigation/loop tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.

- [ ] 11) Split `src/audio/output.rs` into discovery, stream setup, and callback runtime modules
  - ROI/Effort: Medium / M
  - Why it matters: device discovery, stream negotiation, callback mixing, and monitor helpers are still coupled in one platform-sensitive file.
  - Evidence:
    - `src/audio/output.rs` is 696 LOC.
    - `CpalAudioStream` starts at `:188`.
    - Discovery APIs start at `available_hosts` (`:339`) and `available_devices` (`:357`).
    - Stream opening starts at `open_output_stream` (`:412`).
  - Recommended change: separate host/device discovery, stream/config resolution, and callback-runtime state into smaller modules.
  - Risk/tradeoffs: Medium. Audio code is platform-sensitive and callback behavior must stay simple and non-blocking.
  - Suggested validation: audio output tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.

- [ ] 12) Further split app-core browser projection into row-window, panel, chrome, and preload helpers
  - ROI/Effort: Medium / M
  - Why it matters: browser projection is one of the last concentrated app-core/native-shell coupling points, and it still mixes retained-row projection with preload policy and chrome assembly.
  - Evidence:
    - `src/app_core/native_shell/browser_projection.rs` is 540 LOC.
    - `project_browser_panel_frame_model` starts at `:42`.
    - `project_browser_rows_model` starts at `:88`.
    - `preload_browser_window_bpms` starts at `:173`.
  - Recommended change: split rows/panel/chrome/preload into focused helpers and keep tests close to preload-window and row-projection behavior.
  - Risk/tradeoffs: Medium. Browser row ordering, selected-row behavior, and BPM preload ranges must remain stable.
  - Suggested validation: `app_core::native_shell` tests, browser integration tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_quick.sh`.

- [ ] 13) Fix handoff/doc-gardening drift and correct CI contract documentation
  - ROI/Effort: High / S
  - Why it matters: the repository currently contains automation and handoff docs that can direct future agents or contributors to stale plan files and inaccurate CI expectations.
  - Evidence:
    - `docs/plans/active/todo.md` still says the active ordered backlog lives in `tmp/perf_plan.md`, while `AGENTS.md`, `MEMORY.md`, and `tmp/cleanup_plan.md` now treat cleanup as the active confirmed-next backlog.
    - `scripts/fix_trivial_doc_links.sh` rewrites `manual/plan.md`, `manual/todo.md`, and `manual/transient_plan.md` to legacy `docs/plans/*` paths, but `docs/plans/index.md` routes users to `docs/plans/active/*`, and `docs/plans/todo.md` is effectively a dead end.
    - `docs/INDEX.md` describes `scripts/run_agent_request.sh` as executing full local CI even though the script defaults to `devcheck`, and `scripts/ci_local.sh` calls itself CI parity while also running perf guard steps that the GitHub CI workflow does not.
  - Recommended change: align handoff docs on the active cleanup backlog, retarget doc-gardening rewrites to the real active plan entrypoints, and correct CI-entrypoint descriptions so “preflight”, “quick CI”, and “CI parity” each mean exactly one thing.
  - Risk/tradeoffs: Low. The main risk is churn in docs terminology; the fix should prefer accuracy over preserving outdated labels.
  - Suggested validation: `bash scripts/check_docs_index.sh`, `bash scripts/check_markdown_links.sh`, `bash scripts/knowledge_lint.sh`, then `bash scripts/ci_quick.sh`.

- [ ] 14) Decompose `src/app_core/native_bridge.rs` and related metrics/tests into reducer, invalidation, projection, and metrics modules
  - ROI/Effort: Medium / L
  - Why it matters: the retained bridge is now a core runtime subsystem, but action batching, invalidation policy, projection scheduling, and telemetry are still tightly coupled.
  - Evidence:
    - `src/app_core/native_bridge.rs` is 827 LOC.
    - `src/app_core/native_bridge/metrics.rs` is 901 LOC.
    - `src/app_core/native_bridge/tests.rs` is 1253 LOC.
    - `PendingWaveformActions` starts at `src/app_core/native_bridge.rs:110`, while the `NativeAppBridge` implementation begins later in the file.
  - Recommended change: split reducer/coalescer, invalidation policy, projection handoff, and profiling concerns into focused modules, and split tests beside those seams.
  - Risk/tradeoffs: High. This is hot-path runtime code, so coalescing and invalidation behavior must remain identical.
  - Suggested validation: native-bridge tests, browser/projection integration tests, `bash scripts/devcheck.sh`, then `bash scripts/ci_local.sh`.

- [ ] 15) Split `vendor/radiant/src/app/mod.rs` into model, action, and bridge-boundary modules
  - ROI/Effort: Medium / M
  - Why it matters: one large public `radiant` module still defines most of the projection contract, which obscures ownership boundaries and increases import churn.
  - Evidence:
    - `vendor/radiant/src/app/mod.rs` is 1511 LOC.
    - It mixes panel models, `UiAction`, dirty-segment bookkeeping, motion projection, and bridge traits in one public file.
  - Recommended change: split the public app surface into focused modules such as `models`, `actions`, `dirty_segments`, and `bridge`.
  - Risk/tradeoffs: Medium. The main risk is API churn across imports rather than behavior changes.
  - Suggested validation: `radiant` unit tests, `cargo doc -p radiant --no-deps`, then `bash scripts/ci_local.sh`.

- [ ] 16) Finish decomposing `vendor/radiant` native Vello runtime and its test surface
  - ROI/Effort: Medium / L
  - Why it matters: immediate input handling, scene fingerprinting, redraw scheduling, and the large runtime tests are still too coupled for safe incremental work.
  - Evidence:
    - `vendor/radiant/src/gui_runtime/native_vello.rs` is 4571 LOC.
    - `vendor/radiant/src/gui_runtime/native_vello/input.rs` is 1114 LOC.
    - `vendor/radiant/src/gui_runtime/native_vello/tests.rs` is 2321 LOC.
    - Public runtime entrypoints remain at `run_native_vello_app` (`:4511`) and `run_native_vello_app_declarative` (`:4555`) in the same giant module.
  - Recommended change: continue splitting runtime state, fingerprinting, pointer/input routing, redraw scheduling, and tests into smaller behavior-focused modules.
  - Risk/tradeoffs: High. This is hot-path event-loop/render code and easy to regress.
  - Suggested validation: targeted `radiant` runtime tests, perf-sensitive interaction checks, then `bash scripts/ci_local.sh`.

- [ ] 17) Split `vendor/radiant` native-shell state and frame building by responsibility
  - ROI/Effort: Medium / L
  - Why it matters: retained native-shell state still concentrates cached row models, sync-from-model logic, static frame composition, overlay routing, and test scaffolding in a few massive files.
  - Evidence:
    - `vendor/radiant/src/gui/native_shell/state.rs` is 3796 LOC.
    - `vendor/radiant/src/gui/native_shell/state/frame_build.rs` is 2288 LOC and centers the main static-frame builder at `build_frame_with_style_into_with_motion_sinks` (`:6`).
    - `vendor/radiant/src/gui/native_shell/state/tests.rs` is 3773 LOC.
  - Recommended change: split state/cache ownership, model-sync, frame-build, overlay, and browser-row concerns into focused sibling modules, then split tests to match those seams.
  - Risk/tradeoffs: High. This is central retained-scene code, so module moves can regress hit testing, invalidation, or snapshot behavior.
  - Suggested validation: targeted native-shell tests and snapshots, then `bash scripts/ci_local.sh`.

## Progress Log

- 2026-03-09: Phase 1 cleanup audit refreshed from the current `next` workspace state.
- 2026-03-09: Read repository guidance first (`AGENTS.md`, `README.md`, `docs/README.md`, active plans, and `MEMORY.md`) before the audit.
- 2026-03-09: Confirmed the canonical local CI parity command for this environment is `bash scripts/ci_local.sh`.
- 2026-03-09: Completed item 1 in commit `16932de4` and item 2 in commit `1fe099ae`.
- 2026-03-09: Completed item 3 in commit `0b0be54a`.
- 2026-03-09: Completed item 4 in commit `f752dec6`.
- 2026-03-09: Completed item 5 in commit `8d2c30e8`.
