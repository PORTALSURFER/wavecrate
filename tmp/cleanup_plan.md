# Cleanup Plan (ROI Ranked)

Generated: 2026-03-10 (UTC)
Phase: 1 refreshed and revalidated against current `next`, awaiting confirmation
Status legend: `[ ]` pending, `[x]` done
Project language/tooling: Rust 2024 Cargo workspace (`sempal` + `apps/*` + `tools/*` + `vendor/radiant`)
Canonical local CI command: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Audit Notes

- Public API docs are already enforced at the workspace level via `missing_docs = "deny"` in the root and `vendor/radiant` manifests, so the main documentation debt is in naming drift, mixed-responsibility modules, and boundary docs that no longer match current behavior.
- The best cleanup ROI is still concentrated in mixed-responsibility controller/runtime modules, duplicated persistence paths, and giant native-shell/runtime files in `vendor/radiant`.
- The backlog intentionally starts with locally owned, behavior-preserving extractions before the larger `vendor/radiant` runtime splits.
- Phase 2 is paused until the user explicitly confirms sequential implementation from this ordered list.

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
  - Suggested validation: targeted analysis unit tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-09 (`16932de4`)

- [x] 2) Remove duplicated `wav_files` row decoding and upsert SQL in the source DB layer
  - ROI/Effort: High / S
  - Why it matters: persistence fixes currently require synchronized edits across repeated row-decoding and near-identical `INSERT ... ON CONFLICT` blocks.
  - Evidence:
    - `src/sample_sources/db/read.rs` is 393 LOC and repeats `query_map` row decoding at `:20`, `:59`, and `:219`.
    - `src/sample_sources/db/write.rs` is 373 LOC and repeats `INSERT INTO wav_files ... ON CONFLICT(path) DO UPDATE` at `:25`, `:162`, `:198`, `:236`, and `:278`.
  - Recommended change: introduce shared wav-row decode helpers and a small set of explicit upsert builders/helpers instead of open-coded SQL variants.
  - Risk/tradeoffs: Low-medium. Shared helpers must not blur the intentional differences between hash/tag/missing variants.
  - Suggested validation: source DB unit tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
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
  - Suggested validation: browser search unit tests for sync and worker paths, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
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
  - Suggested validation: focused map query tests, map-view controller tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
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
  - Suggested validation: waveform-loading tests, cache-loading controller tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-09 (`8d2c30e8`)

- [x] 6) Isolate clipboard paste/import into an explicit staged workflow and add real failure-path coverage
  - ROI/Effort: High / M
  - Why it matters: clipboard import performs filesystem mutation plus journal-backed bookkeeping, but most of the workflow still sits in one large background job body.
  - Evidence:
    - `src/app/controller/ui/clipboard_paste.rs` is 491 LOC.
    - `run_clipboard_paste_job` starts at `:239` and owns validation, naming, staging, copy, DB writes, finalize rename, cleanup, progress updates, and summary creation.
    - The current local tests only cover `validate_relative_folder_path` (`:476-491`), not the multi-stage rollback/error paths.
  - Recommended change: model the paste job as explicit stages/results, then add tests for rollback and partial-failure paths without requiring full controller orchestration.
  - Risk/tradeoffs: Medium. Journal/finalize behavior must remain exact or imports could leave stale staged files behind.
  - Suggested validation: clipboard/external-drop tests, staged failure-path tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-09 (`30d25841`)

- [x] 7) Split `src/sample_sources/db/file_ops_journal.rs` into entry, store, and reconciliation modules
  - ROI/Effort: High / M
  - Why it matters: this is crash-recovery code, so clearer boundaries improve reviewability and reduce startup-recovery risk.
  - Evidence:
    - `src/sample_sources/db/file_ops_journal.rs` is 628 LOC.
    - Entry/storage helpers and reconciliation are mixed together: `remove_entry` (`:293`), `list_entries` (`:301`), `reconcile_pending_ops` (`:457`), `reconcile_entry` (`:497`), and `reconcile_source_entry` (`:573`).
    - Tests already live in `src/sample_sources/db/file_ops_journal/tests.rs`, which makes a production split easier.
  - Recommended change: separate journal entry/schema types, persistence API, and reconciliation executor behavior into focused sibling modules.
  - Risk/tradeoffs: Medium. Startup recovery semantics and journal cleanup must remain byte-for-byte compatible in behavior.
  - Suggested validation: journal unit tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-09 (`08541a52`, follow-up `d538fd60`)

- [x] 8) Prune or explicitly justify remaining non-vendor `#[allow(dead_code)]` and `#[allow(clippy::too_many_arguments)]` suppressions
  - ROI/Effort: Medium-High / S
  - Why it matters: the remaining suppressions make it harder to distinguish intentional compatibility shims from simply stale code.
  - Evidence:
    - There are 32 non-test suppressions under `src/`, `apps/`, and `tools/`.
    - Representative examples include `src/app/controller/library/wavs/waveform_loading.rs:189`, `src/app/controller/ui/loading.rs:114`, `src/app/controller/jobs/messages.rs:162`, `src/app/controller/state/cache.rs:86`, `src/sample_sources/scanner/scan/runner.rs:68`, `src/updater/github.rs:29`, and `apps/installer/src/install.rs:17`.
  - Recommended change: remove truly unused code, move test-only helpers behind `#[cfg(test)]`, and add short rationale comments only where compatibility or platform glue requires the suppression.
  - Risk/tradeoffs: Low-medium. A few suppressed fields may still be intentionally retained for serialization/platform integration.
  - Suggested validation: `cargo clippy --workspace --all-targets`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-09 (`b5702240`)

- [x] 9) Decompose destructive selection-edit workflows into focused write services and thin controller orchestration
  - ROI/Effort: High / M
  - Why it matters: destructive edits still combine sample-buffer transforms, file writes, DB updates, undo capture, cache refreshes, and playback follow-up in one module.
  - Evidence:
    - `src/app/controller/library/selection_edits/mod.rs` is 551 LOC.
    - `crop_waveform_selection_to_new_sample` starts at `:148`.
    - `normalize_waveform_selection` starts at `:280`.
    - `reverse_waveform_selection` starts at `:337`.
  - Recommended change: extract edit-application services and keep controller methods as orchestration shells over those services.
  - Risk/tradeoffs: Medium. These flows touch both files and persisted state, so regression risk is real without focused tests.
  - Suggested validation: waveform-edit controller tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-09 (`07afb548`)

- [x] 10) Split `src/app/controller/playback/transport.rs` by interaction domain
  - ROI/Effort: Medium-High / M
  - Why it matters: selection dragging, loop toggle policy, seeking, BPM snapping, and playback follow-up still live in one dense controller file.
  - Evidence:
    - `src/app/controller/playback/transport.rs` is 965 LOC.
    - Selection drag entrypoints begin at `:19`.
    - Loop toggle logic starts at `:192`.
    - Playback helpers continue through the later half of the file, while tests also live in the same module tail.
  - Recommended change: extract `selection_drag`, `looping`, `seek`, `bpm_snap`, and playback helpers into focused modules with tests aligned to those seams.
  - Risk/tradeoffs: Medium. Playback and selection edge cases are user-visible and need stable coverage during the split.
  - Suggested validation: transport-focused tests, waveform navigation/loop tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-09 (`1a0a20eb`)

- [x] 11) Split `src/audio/output.rs` into discovery, stream setup, and callback runtime modules
  - ROI/Effort: Medium / M
  - Why it matters: device discovery, stream negotiation, callback mixing, and monitor helpers are still coupled in one platform-sensitive file.
  - Evidence:
    - `src/audio/output.rs` is 696 LOC.
    - `CpalAudioStream` starts at `:188`.
    - Discovery APIs start at `available_hosts` (`:339`) and `available_devices` (`:357`).
    - Stream opening starts at `open_output_stream` (`:412`).
  - Recommended change: separate host/device discovery, stream/config resolution, and callback-runtime state into smaller modules.
  - Risk/tradeoffs: Medium. Audio code is platform-sensitive and callback behavior must stay simple and non-blocking.
  - Suggested validation: audio output tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-09 (`bb7216dd`)

- [x] 12) Further split app-core browser projection into row-window, panel, chrome, and preload helpers
  - ROI/Effort: Medium / M
  - Why it matters: browser projection is one of the last concentrated app-core/native-shell coupling points, and it still mixes retained-row projection with preload policy and chrome assembly.
  - Evidence:
    - `src/app_core/native_shell/browser_projection.rs` is 540 LOC.
    - `project_browser_panel_frame_model` starts at `:42`.
    - `project_browser_rows_model` starts at `:88`.
    - `preload_browser_window_bpms` starts at `:173`.
  - Recommended change: split rows/panel/chrome/preload into focused helpers and keep tests close to preload-window and row-projection behavior.
  - Risk/tradeoffs: Medium. Browser row ordering, selected-row behavior, and BPM preload ranges must remain stable.
  - Suggested validation: `app_core::native_shell` tests, browser integration tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-09 (`bceaaeeb`)

- [x] 13) Fix handoff/doc-gardening drift and correct CI contract documentation
  - ROI/Effort: High / S
  - Why it matters: the repository currently contains automation and handoff docs that can direct future agents or contributors to stale plan files and inaccurate CI expectations.
  - Evidence:
    - `docs/plans/active/todo.md` still says the active ordered backlog lives in `tmp/perf_plan.md`, while `AGENTS.md`, `MEMORY.md`, and `tmp/cleanup_plan.md` now treat cleanup as the active confirmed-next backlog.
    - `scripts/fix_trivial_doc_links.sh` rewrites `manual/plan.md`, `manual/todo.md`, and `manual/transient_plan.md` to legacy `docs/plans/*` paths, but `docs/plans/index.md` routes users to `docs/plans/active/*`, and `docs/plans/todo.md` is effectively a dead end.
    - `docs/INDEX.md` describes `scripts/run_agent_request.sh` as executing full local CI even though the script defaults to `devcheck`, and `scripts/ci_local.sh` calls itself CI parity while also running perf guard steps that the GitHub CI workflow does not.
  - Recommended change: align handoff docs on the active cleanup backlog, retarget doc-gardening rewrites to the real active plan entrypoints, and correct CI-entrypoint descriptions so “preflight”, “quick CI”, and “CI parity” each mean exactly one thing.
  - Risk/tradeoffs: Low. The main risk is churn in docs terminology; the fix should prefer accuracy over preserving outdated labels.
  - Suggested validation: docs index/link checks if touched, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-09 (`319cefdd`)

- [x] 14) Split waveform selection/edit-fade action logic out of `src/app/controller/playback/waveform_actions.rs`
  - ROI/Effort: High / M
  - Why it matters: play-selection, edit-selection, fade-handle, smart-scale, BPM-snap, and drag-state transitions are concentrated in one 973 LOC controller file, which keeps recent waveform fixes expensive and easy to regress.
  - Evidence:
    - `src/app/controller/playback/waveform_actions.rs:119`, `:128`, `:169`, and `:208` mix selection dragging, edge policy, smart-scale, and edit-range preservation in the same file.
    - Fade-handle and edit-range mutation logic still spreads through `:263-417`, `:555-775`, and `:784-944`.
    - Recent behavior changes in this area have stacked more hotkey/native-action routing into the same file.
  - Recommended change: extract focused modules for selection dragging, edit-mark handling, fade adjustment, and BPM-snap/smart-scale helpers, and keep the top-level controller entrypoints as thin orchestrators.
  - Risk/tradeoffs: Medium. This code is heavily user-visible, so behavior must remain stable while the seams move.
  - Suggested validation: waveform action unit tests, focused playback/controller tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-10 (`002ce1b9`)

- [x] 15) Decompose `src/app_core/native_bridge.rs` and split the catch-all native-bridge test surface
  - ROI/Effort: High / M-L
  - Why it matters: native action reduction, invalidation classification, projection scheduling, and telemetry still sit behind one 858 LOC bridge entrypoint plus a 1342 LOC test hub, which slows any runtime work and obscures ownership.
  - Evidence:
    - `src/app_core/native_bridge.rs:146-314`, `:447-596`, and `:729-834` still mix queued reduction, immediate application, model pulling, and frame bookkeeping in one module.
    - `src/app_core/native_bridge/tests.rs` remains a broad end-to-end assertion sink rather than feature-aligned test modules, and it has grown to 1342 LOC.
    - `src/app_core/native_bridge/metrics.rs` is also 863 LOC, amplifying the same concentration.
  - Recommended change: split reducer/coalescing, invalidation, projection handoff, and metrics/reporting into explicit modules, then move tests beside those seams instead of continuing to grow one shared harness.
  - Risk/tradeoffs: High. This is hot-path runtime code, so batching and invalidation behavior cannot drift.
  - Suggested validation: targeted native-bridge tests, projection cache tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-10

- [ ] 16) Split source DB schema/migration logic and add dedicated migration fixtures
  - ROI/Effort: High / M
  - Why it matters: schema creation and opportunistic migrations are still bundled together, which makes persistence evolution hard to review and leaves migration behavior under-tested.
  - Evidence:
    - `src/sample_sources/db/schema.rs` concentrates base DDL and every follow-on migration step.
    - The base schema starts at `:21`, while `remove_invalid_relative_paths` (`:186`), `ensure_wav_files_optional_columns` (`:219`), `ensure_analysis_jobs_optional_columns` (`:294`), and `ensure_samples_optional_columns` (`:361`) all mutate live DBs in one file.
    - DB-local specialty tests are still sparse compared with the migration surface.
  - Recommended change: split base schema, migration steps, and migration helpers into dedicated modules, and add fixture-driven migration tests that exercise representative old DB states.
  - Risk/tradeoffs: Medium. SQLite migration order and idempotence have to remain exact.
  - Suggested validation: source DB migration tests with fixtures, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 17) Unify duplicated source DB write mutation paths around `SourceWriteBatch`
  - ROI/Effort: High / M
  - Why it matters: write operations still expose overlapping one-shot and batch mutation APIs, so new persistence behavior needs repeated SQL and setter changes.
  - Evidence:
    - `src/sample_sources/db/write.rs` still carries near-duplicate SQL constants at `:8`, `:15`, `:23`, and `:31`.
    - One-shot versus batch mutation flows diverge around `:98` and `:238`.
    - Repeated setters exist around `:125`, `:130`, `:144` and again around `:340`, `:351`, `:363`.
  - Recommended change: make `SourceWriteBatch` the primary mutation path, keep thin convenience wrappers on top, and centralize shared SQL/update builders so new fields are defined once.
  - Risk/tradeoffs: Medium. Refactoring write paths can accidentally change transaction shape or missing-value behavior.
  - Suggested validation: source DB write tests, targeted rating/tag persistence tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 18) Reduce `src/app/controller.rs` to controller-root responsibilities only
  - ROI/Effort: High / M
  - Why it matters: the root controller still mixes construction, dispatch, high-level orchestration, and feature-specific helpers, which makes boundary ownership unclear and encourages more growth in the root file.
  - Evidence:
    - `src/app/controller.rs:177-239`, `:254-312`, `:324-454`, and `:479-504` still combine construction, perf governor bookkeeping, status handling, undo/redo flow, and feature-controller accessors.
    - The file remains one of the densest non-vendor controller entrypoints after the recent feature additions.
  - Recommended change: keep `controller.rs` focused on type definition, construction, and top-level delegation, while moving feature-specific routing or helpers into the existing domain modules under `src/app/controller/`.
  - Risk/tradeoffs: Medium. Dispatch boundaries and borrow flow need to stay clear during the split.
  - Suggested validation: controller-focused unit tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 19) Split similarity resolution orchestration from repository, math, and job-enqueue code
  - ROI/Effort: High / M
  - Why it matters: sample similarity resolution still concentrates candidate gathering, repository lookups, score shaping, and background-job coordination in one place.
  - Evidence:
    - `src/app/controller/library/wavs/similar/resolve.rs` is now 531 LOC and still branches heavily across the pipeline around `:56`, `:98`, `:111`, `:148`, `:238`, and `:276`.
    - The file owns both domain policy and the low-level steps needed to compute results.
  - Recommended change: split repository access, score composition, and job coordination into smaller modules, leaving `resolve.rs` as a clear orchestration layer or replacing it with a small module tree.
  - Risk/tradeoffs: Medium. Similarity ordering and fallback behavior must remain stable.
  - Suggested validation: similar-sample controller tests, background-job polling tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 20) Split `vendor/radiant` native-shell state and frame composition by responsibility
  - ROI/Effort: High / L
  - Why it matters: retained native-shell state still concentrates cached row models, sync-from-model logic, frame building, overlay routing, and test scaffolding in a few massive files.
  - Evidence:
    - `vendor/radiant/src/gui/native_shell/state.rs` is 3948 LOC.
    - `vendor/radiant/src/gui/native_shell/state/frame_build.rs` is 2492 LOC.
    - `vendor/radiant/src/gui/native_shell/state/tests.rs` is 4823 LOC.
  - Recommended change: split state/cache ownership, model sync, frame build, overlays, and row-specific rendering into focused sibling modules, then split tests to match those seams.
  - Risk/tradeoffs: High. This is central retained-scene code, so hit-testing, invalidation, and snapshot behavior can regress if the split is sloppy.
  - Suggested validation: targeted `radiant` native-shell tests run serially, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 21) Finish decomposing `vendor/radiant` native Vello runtime, input routing, and test surface
  - ROI/Effort: High / L
  - Why it matters: immediate input handling, drag state, scene fingerprinting, redraw scheduling, and the runtime tests are still too coupled for safe incremental changes.
  - Evidence:
    - `vendor/radiant/src/gui_runtime/native_vello.rs` is 4680 LOC.
    - `vendor/radiant/src/gui_runtime/native_vello/input.rs` is 1302 LOC.
    - `vendor/radiant/src/gui_runtime/native_vello/tests.rs` is 2678 LOC.
  - Recommended change: continue splitting runtime state, input/pointer routing, redraw scheduling, and scene diff/fingerprint logic into behavior-focused modules with matching tests.
  - Risk/tradeoffs: High. This is hot-path event-loop/render code and easy to regress under real interaction.
  - Suggested validation: targeted `radiant` runtime/input tests run serially, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 22) Split the public `radiant::app` contract surface and add boundary docs
  - ROI/Effort: Medium-High / M
  - Why it matters: one large public `radiant` module still defines most of the projection contract, which obscures ownership boundaries and makes the bridge/UI contract harder to understand.
  - Evidence:
    - `vendor/radiant/src/app/mod.rs` is 1635 LOC.
    - It currently mixes panel models, `UiAction`, dirty-segment bookkeeping, motion projection, and bridge traits in one public module.
    - The file still carries broad suppressions such as `#[allow(dead_code)]` near `:104`.
  - Recommended change: split the public app surface into focused modules such as `models`, `actions`, `dirty_segments`, and `bridge`, and add short module docs that explain how the host and native shell interact.
  - Risk/tradeoffs: Medium. The main risk is API churn across imports rather than behavior change.
  - Suggested validation: `radiant` unit tests, `cargo doc -p radiant --no-deps`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 23) Break up giant test hubs to match production seams
  - ROI/Effort: Medium-High / M
  - Why it matters: several test files have grown into catch-all regression bins, which slows navigation and makes it harder to see coverage gaps by feature.
  - Evidence:
    - `src/app_core/native_bridge/tests.rs` is 1342 LOC.
    - `src/app_core/native_shell/tests.rs` is 1054 LOC.
    - `src/app/controller/tests/browser_actions.rs` and `src/app/controller/tests/folders_core.rs` are still large multi-domain hubs.
    - `vendor/radiant/src/gui/native_shell/state/tests.rs` (4823 LOC) and `vendor/radiant/src/gui_runtime/native_vello/tests.rs` (2678 LOC) are still multi-thousand-line umbrellas.
  - Recommended change: move tests next to the production seams created by the earlier refactors, and split remaining integration-style tests by behavior domain instead of piling onto one file.
  - Risk/tradeoffs: Low-medium. Mostly structural, but careless moves can weaken discoverability if the new module layout is inconsistent.
  - Suggested validation: affected unit tests run serially where Rust test binaries overlap, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 24) Clarify `AsyncSource` worker lifecycle and teardown contract
  - ROI/Effort: Medium-High / M
  - Why it matters: async decode still spawns a worker thread without explicit join ownership, which makes teardown intent and failure behavior harder to reason about.
  - Evidence:
    - `src/audio/async_decode.rs:18` defines the worker state.
    - The decode thread is spawned around `:57`.
    - `Drop` at `:191` only flips the stop flag; there is no retained `JoinHandle` or explicit shutdown contract.
  - Recommended change: document the lifecycle explicitly, retain the worker handle or justify detached behavior, and add tests for stop/shutdown expectations.
  - Risk/tradeoffs: Medium. Joining on drop can deadlock if the worker waits on callbacks, so the shutdown contract must be explicit.
  - Suggested validation: async decode tests, audio-related unit tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 25) Split playback transport/player lifecycle helpers in `src/app/controller/playback/player.rs`
  - ROI/Effort: Medium / M
  - Why it matters: player lifecycle, transport-follow-up, waveform sync, and selection-aware playback helpers still live together, which keeps playback changes broad.
  - Evidence:
    - `src/app/controller/playback/player.rs` is 446 LOC and still spreads concerns across `:10`, `:182`, `:219`, `:325`, and `:374`.
    - The file mixes transport intent, playback setup, and follow-on UI synchronization work.
  - Recommended change: separate transport-facing playback helpers, player-state update helpers, and UI follow-up behaviors into smaller modules with clear docs.
  - Risk/tradeoffs: Medium. Playback sequencing and loop/focus interactions are user-visible and easy to regress.
  - Suggested validation: playback controller tests, loop/transport tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 26) Split native-shell style tokens into palette, sizing, and tier policy
  - ROI/Effort: Medium / M
  - Why it matters: visual tokens and scaling policy are still clustered together, which makes future UI cleanup harder and blurs which values are semantic colors versus layout decisions.
  - Evidence:
    - `vendor/radiant/src/gui/native_shell/style.rs` is 938 LOC and remains a broad token file.
    - Related layout/state code still imports it as a catch-all dependency.
  - Recommended change: separate semantic palette values, sizing constants, and tier/scaling policy into focused modules with small docs on intended usage.
  - Risk/tradeoffs: Low-medium. Mostly structural, but token moves can create churn across imports.
  - Suggested validation: `radiant` style/native-shell tests run serially, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 27) Deduplicate `tools/analysis-admin` CLI scaffolding and add parser tests
  - ROI/Effort: Medium / S
  - Why it matters: the analysis admin binaries repeat argument parsing and `main`/`run` scaffolding, which makes small tooling fixes noisy.
  - Evidence:
    - Repeated CLI entry scaffolding appears in `tools/analysis-admin/src/bin/sempal-hdbscan.rs`, `sempal-umap.rs`, `sempal-ann-rebuild.rs`, and `sempal-db-inspect.rs`.
    - The binaries currently follow the same manual `parse_args` / `run` pattern with only small command-specific differences.
  - Recommended change: extract a tiny shared CLI support layer for common parsing/runtime boilerplate and add focused parser tests around each command's unique flags.
  - Risk/tradeoffs: Low. Tooling-only changes, but CLI error text should remain stable where scripts may rely on it.
  - Suggested validation: analysis-admin tool tests, command help smoke checks, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 28) Resolve UMAP/t-SNE naming drift across public APIs, CLI, and status text
  - ROI/Effort: Medium / S
  - Why it matters: the codebase still exposes UMAP names in places that actually talk to t-SNE-backed data or status messages, which makes the product language inconsistent.
  - Evidence:
    - `src/app/controller/ui/map_view/repository.rs:49`, `:81`, `:117`, and `:210` still reflect the older naming shape.
    - `src/app/controller/ui/status_message.rs:65`, `src/app/controller/library/background_jobs/polling/runtime_handlers.rs:23`, and `src/app/controller/library/similarity_prep/runtime.rs:202` still use public-facing terms that drift.
    - `src/analysis/umap.rs` and `tools/analysis-admin/src/bin/sempal-umap.rs` keep the same mismatch at the analysis/tool layer.
  - Recommended change: pick the canonical user-facing term, align status text and public docs on it, and isolate any legacy/internal compatibility naming behind clearly documented adapters.
  - Risk/tradeoffs: Low-medium. Renaming must avoid breaking persisted schema names or external tool expectations without a compatibility plan.
  - Suggested validation: map-view/status tests, analysis/tooling smoke tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 29) Replace the live STFT complexity suppression with a context/scratch struct
  - ROI/Effort: Medium / S-M
  - Why it matters: one of the remaining `too_many_arguments` suppressions is on live DSP code, which signals a real readability issue rather than legacy glue.
  - Evidence:
    - `src/analysis/frequency_domain/stft.rs:3` carries `#[allow(clippy::too_many_arguments)]`.
    - `src/analysis/frequency_domain/stft.rs` is 414 LOC.
    - `compute_frames` starts at `:37`.
    - `process_frame` starts at `:106`.
  - Recommended change: introduce a small typed context/scratch container for the repeated frame-processing inputs so the STFT flow becomes easier to read and test without changing math behavior.
  - Risk/tradeoffs: Low-medium. DSP code is sensitive to accidental allocation or ordering changes, so the refactor should stay purely structural.
  - Suggested validation: STFT unit tests or new focused math tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 30) Prune or justify the remaining broad vendor/runtime suppressions and dead layout surface
  - ROI/Effort: Low-Medium / S
  - Why it matters: after the larger structural items land, the remaining broad `allow(dead_code)` / layout compatibility surface in `vendor/radiant` will be easier to evaluate and either remove or document.
  - Evidence:
    - Remaining suppressions appear in `vendor/radiant/src/app/mod.rs`, `vendor/radiant/src/gui/layout_core/model.rs`, `vendor/radiant/src/gui/layout_core/engine/mod.rs`, `vendor/radiant/src/gui/native_shell/layout.rs`, `vendor/radiant/src/gui/native_shell/state.rs`, and `vendor/radiant/src/gui/types.rs`.
    - Several of those files also look like compatibility or facade surfaces rather than active domain logic.
  - Recommended change: after the module splits, remove dead surface area where possible and add short rationale comments only for the suppressions that remain intentionally public or compatibility-driven.
  - Risk/tradeoffs: Low. This is mostly cleanup, but it depends on earlier refactors making ownership clearer first.
  - Suggested validation: `cargo clippy -p radiant --all-targets`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

## Progress Log

- 2026-03-10: Refreshed the cleanup audit against the current `next` head and reordered the remaining work into a 30-item strict ROI backlog.
- 2026-03-10: Revalidated the remaining top hotspots after the latest waveform/native-shell changes; item 14 remains the highest-ROI next step and the evidence counts above now match the current head.
- 2026-03-10: Completed item 14 by splitting `waveform_actions` into focused `shared`, `selection_updates`, `edit_selection`, and `edit_fades` modules while keeping the `AppController` waveform facade stable.
- 2026-03-10: Completed item 15 by splitting `native_bridge` into batching, reducer, and projection-runtime modules and moving the native-bridge tests into `queue`, `bridge_runtime`, and `projection_cache` groups.
- 2026-03-10: Read repository guidance first (`AGENTS.md`, `README.md`, `docs/README.md`, `docs/plans/index.md`, `docs/plans/active/runtime_performance_exec_plan.md`, `docs/plans/active/todo.md`, `docs/plans/active/cleanup_architecture_note.md`, and `MEMORY.md`) before refreshing the plan.
- 2026-03-10: Confirmed the canonical local CI parity command for this current Windows environment is `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
- 2026-03-10: Audit evidence was gathered from `src/app`, `src/app_core`, `src/sample_sources`, `src/audio`, `src/analysis`, `tools/analysis-admin`, and `vendor/radiant`, plus targeted file-size and suppression scans.
- 2026-03-10: Phase 2 implementation remains paused pending explicit user confirmation to start at item 14.
- 2026-03-09: Completed item 1 in commit `16932de4` and item 2 in commit `1fe099ae`.
- 2026-03-09: Completed item 3 in commit `0b0be54a`.
- 2026-03-09: Completed item 4 in commit `f752dec6`.
- 2026-03-09: Completed item 5 in commit `8d2c30e8`.
- 2026-03-09: Completed item 6 in commit `30d25841`.
- 2026-03-09: Completed item 7 in commits `08541a52` and `d538fd60`.
- 2026-03-09: Completed item 8 in commit `b5702240`.
- 2026-03-09: Completed item 9 in commit `07afb548`.
- 2026-03-09: Completed item 10 in commit `1a0a20eb`.
- 2026-03-09: Completed item 11 in commit `bb7216dd`.
- 2026-03-09: Completed item 12 in commit `bceaaeeb`.
- 2026-03-09: Completed item 13 in commit `319cefdd`.
