# Improvement Audit Plan

- Generated: `2026-03-15`
- Status: `Phase 1 complete - awaiting explicit implementation confirmation`
- Branch: `next`
- Audit baseline commit: `74e6f51b`
- Canonical Windows validation commands:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Repository Context

- Project purpose: `sempal` is an audio sample triage tool centered on exploration, auditioning, curation, and responsive interaction.
  - Intent label: Explicitly documented
  - Evidence: `README.md`, `docs/design_principles.md`
- Maturity level: a mature Rust workspace with strong local/CI guardrails, an actively owned vendor UI framework, and multiple completed execution-plan lanes.
  - Intent label: Strongly implied by code/docs
  - Evidence: `Cargo.toml`, `.github/workflows/ci.yml`, `docs/TEST.md`, `docs/plans/index.md`
- Primary languages / frameworks / tooling: Rust 2024 workspace, `vendor/radiant` native GUI framework, Vello/Winit runtime, rusqlite-backed sample DBs, nextest, and PowerShell/Bash validation wrappers.
  - Intent label: Explicitly documented
  - Evidence: `Cargo.toml`, `README.md`, `docs/ARCHITECTURE.md`, `docs/TEST.md`
- Repository shape: core app/domain code in `src/`, app-core/native projection in `src/app_core`, vendor GUI/runtime code in `vendor/radiant/`, plus layered docs and active/parked plans under `docs/plans` and `tmp/`.
  - Intent label: Explicitly documented
  - Evidence: `docs/ARCHITECTURE.md`, `docs/README.md`, `Cargo.toml`
- Architectural boundaries: `src/` owns domain logic and app behavior, `src/app_core` owns the stable projection/action bridge, and `vendor/radiant` owns widget behavior, layout, input routing, diff/update logic, and native runtime orchestration.
  - Intent label: Explicitly documented
  - Evidence: `README.md`, `docs/ARCHITECTURE.md`
- Test strategy: tight local compile loops (`devcheck`), pre-commit quick validation (`ci_quick`), broader parity validation (`ci_local`), direct `vendor/radiant` tests, and a GUI contract stack that is intentionally broader than default CI.
  - Intent label: Explicitly documented
  - Evidence: `docs/TEST.md`, `.github/workflows/ci.yml`, `docs/gui_test_platform.md`
- Documented priorities: responsiveness, non-blocking behavior, deterministic interaction, explicit ownership boundaries, and incremental reduction of oversized responsibility hubs.
  - Intent label: Explicitly documented
  - Evidence: `docs/design_principles.md`, `docs/QUALITY_SCORE.md`, `docs/file_size_budget_allowlist.txt`, `tmp/cleanup_audit_hotspots.md`
- Explicit non-goals: not a DAW replacement, not a cloud/social platform, and not a visually ornamental app at the expense of responsiveness.
  - Intent label: Explicitly documented
  - Evidence: `docs/design_principles.md`

## Ordered ROI Backlog

### [x] 1. Add direct `SourceDatabase` write-contract coverage and narrow `db/write.rs` around batch mutation helpers
- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Completed: `2026-03-15`
- Commit: `f0ddc97d` (`test(db): localize source write contracts`)
- Assumption used: the safest scope is to keep the public `SourceDatabase`/`SourceWriteBatch` API stable, move write-contract tests next to the implementation, and split only the internal upsert/mutation helpers.
- Validation:
  - `cargo test sample_sources::db::write::tests -- --test-threads=1`
  - `cargo test sample_sources::db::source_db_mod_tests -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Why it matters: this repository explicitly warns that bugs can rename, modify, or delete sample-library state. `db/write.rs` is the central mutation surface for the per-source SQLite catalog, yet its behavior is mostly exercised indirectly through other workflows.
- Evidence:
  - `README.md` warns that the app can modify, rename, or delete files and advises backups.
  - `src/sample_sources/db/write.rs` is allowlisted in `docs/file_size_budget_allowlist.txt` and appears as a hotspot in `tmp/cleanup_audit_hotspots.md`.
  - `src/sample_sources/db/write.rs:39`, `:56`, `:77`, `:155`, `:184`, and `:242` show `ContentHashPolicy`, `TagPolicy`, `WavFileWriteSpec`, `execute_wav_upsert`, `impl SourceDatabase`, and `write_batch` living in one file.
  - `rg` finds no local `#[cfg(test)]` block in `src/sample_sources/db/write.rs`.
  - Existing direct references are mostly indirect through `src/sample_sources/db/file_ops_journal/tests.rs`, which exercises journal flows rather than the full write-contract matrix for overwrite, remove, tag/lock/missing toggles, and batched commit boundaries.
- Recommended change: add focused direct tests around `SourceWriteBatch` and `SourceDatabase` mutation invariants, then split the file into smaller internal helpers for write specs, statement execution, and batch mutation orchestration while keeping the public API stable.
- Expected impact: tighter protection around a destructive-adjacent persistence surface and smaller review scope for future DB mutation changes.
- Risks / tradeoffs: test fixtures will need temp DB setup and clear assertions around transaction boundaries; the refactor should not broaden the current API.
- Dependencies: none
- Suggested validation:
  - targeted `sample_sources::db` tests covering batch commit, overwrite, remove, and tag/missing/lock mutations
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 2. Decompose `vendor/radiant/src/gui_runtime/native_vello/runtime_startup.rs` into startup policy, window/surface bring-up, and first-frame sequencing helpers
- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Completed: `2026-03-15`
- Commit: `daa8eb46` in `vendor/radiant` (`refactor(runtime): split startup helpers`), recorded in root at `b2f93e98` (`refactor(runtime): record startup helper split`)
- Assumption used: startup policy, layout/first-frame sequencing, and the broader bring-up structure can be separated safely, but the render-surface fallback lifetime path should remain inline inside `initialize_runtime`.
- Validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml startup -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Why it matters: this is a hot native-runtime entrypoint that currently couples runner construction, window attributes, GPU surface fallback behavior, startup reveal timing, layout prep, and placeholder-scene logic. That mix increases regression risk in the first-frame path.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_startup.rs` is allowlisted in `docs/file_size_budget_allowlist.txt`.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_startup.rs:22`, `:158`, `:181`, `:305`, `:363`, and `:370` show `new`, `build_window_attributes`, `initialize_runtime`, `prepare_startup_first_frame_scene`, `arm_startup_reveal_deadline`, and `build_startup_placeholder_scene` in one file.
  - The file is 400+ lines and has no local `#[cfg(test)]` block.
  - `README.md` and `docs/design_principles.md` emphasize responsiveness and non-blocking startup behavior, which makes this first-frame sequencing path especially sensitive.
- Recommended change: preserve the existing runner API and startup behavior, but split startup policy, window/surface initialization, and first-frame sequencing into focused helpers/modules with clearer ownership and direct characterization coverage where practical.
- Expected impact: safer changes in the startup path, better separation between startup policy and GPU/window fallback code, and smaller diffs for future runtime work.
- Risks / tradeoffs: startup timing and surface fallback order are delicate; the refactor needs to stay behavior-preserving.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 3. Split browser row windowing into hit-testing, viewport bounds, row projection, and scrollbar helpers
- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Completed: `2026-03-15`
- Commit: `4cc6cde9` in `vendor/radiant` (`refactor(browser): split row windowing helpers`)
- Assumption used: the safest split is local helper extraction that preserves the existing `windowing::*` export surface, because other native-shell modules already depend on these helpers by name.
- Validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_scrollbars -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_rows -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Why it matters: browser row windowing is a correctness-sensitive surface for virtualization, pointer targeting, and scroll behavior. The current file mixes those concerns in one allowlisted module, which makes subtle browser regressions harder to reason about.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/browser_rows/windowing.rs` is allowlisted in `docs/file_size_budget_allowlist.txt`.
  - `vendor/radiant/src/gui/native_shell/state/browser_rows/windowing.rs:5`, `:50`, `:163`, `:290`, and `:368` show `row_index_for_visible_rows`, `browser_rows_cache_key`, `rendered_browser_rows_cached_with_window_start_and_previous`, `browser_scrollbar_layout`, and `browser_rows_window_bounds_with_previous` in one file.
  - Existing tests are spread across `vendor/radiant/src/gui/native_shell/state/tests/browser_rows/hit_testing.rs` and `vendor/radiant/src/gui/native_shell/state/tests/browser_scrollbars.rs`, which confirms the behavior is important but leaves the production ownership surface broad.
  - `docs/design_principles.md` and `README.md` emphasize deterministic input behavior, which directly applies to row hit-testing and scrollbar mapping.
- Recommended change: separate pure row-window math, hit-testing, rendered-row projection, and scrollbar layout/drag helpers into smaller modules and add focused viewport-preservation tests around the extracted boundaries.
- Expected impact: lower regression risk in browser virtualization and clearer ownership between cached row projection and pointer/scrollbar behavior.
- Risks / tradeoffs: row-window preservation and scrollbar clamping are visually subtle; tests need to lock in current semantics before extraction.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_rows -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_scrollbars -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 4. Separate overlay geometry accessors from overlay-family rendering in `vendor/radiant/src/gui/native_shell/state/overlays.rs`
- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Completed: `2026-03-15`
- Commit: `e017b9d7` in `vendor/radiant` (`refactor(overlays): split geometry and render helpers`)
- Assumption used: shared geometry and border helpers should remain the common native-shell access surface, while prompt/progress/drag paint paths can move into family-specific modules without changing overlay call sites.
- Validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml overlays -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml overlay_controls -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Why it matters: progress, prompt, and drag overlays are user-visible interruption states. The current file combines layout rect accessors and three distinct overlay render flows, which makes overlay-specific behavior harder to discover and change safely.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/overlays.rs` is allowlisted in `docs/file_size_budget_allowlist.txt`.
  - `vendor/radiant/src/gui/native_shell/state/overlays.rs:5`, `:21`, `:33`, `:52`, `:56`, `:217`, and `:451` show shared geometry helpers plus `render_progress_overlay`, `render_confirm_prompt`, and `render_drag_overlay` in one file.
  - Existing tests in `vendor/radiant/src/gui/native_shell/state/tests/overlays.rs` cover some overlay behavior, but they are broad UI assertions rather than ownership-focused tests around prompt/progress/drag geometry seams.
  - The file is one of the largest remaining `vendor/radiant` state modules with no local tests alongside the implementation.
- Recommended change: keep behavior stable while extracting overlay-family-specific rendering helpers and shared geometry/layout accessors into smaller modules with clearer doc comments and targeted tests.
- Expected impact: easier maintenance of prompt/progress/drag behavior and smaller blast radius for overlay changes.
- Risks / tradeoffs: overlay visuals are easy to change accidentally; test coverage should stay behavior-oriented rather than snapshotting implementation detail.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml overlays -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 5. Decompose `src/app/controller/library/wavs/browser_pipeline.rs` into explicit sync stages with direct local stage tests
- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Completed: `2026-03-15`
- Assumption used: the sync browser pipeline should stay execution-model-specific while exposing explicit base, folder-acceptance, and visible-row stage helpers that match the existing worker semantics only where those semantics are already shared.
- Validation:
  - `cargo test browser_pipeline -- --test-threads=1`
  - `cargo test folder_filter_visible_rows_match_sync_pipeline -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Why it matters: the synchronous browser visible-row pipeline still owns base-stage refresh, folder acceptance, filtering, scoring, sorting, and final visible-row materialization in one file. That is a direct maintainability risk on one of the app’s most central interaction paths.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `build_visible_rows` in `src/app/controller/library/wavs/browser_pipeline.rs:67` as a large function hotspot and lists the file itself among oversized modules.
  - `src/app/controller/library/wavs/browser_pipeline.rs:67`, `:253`, `:311`, and `:379` show `build_visible_rows`, `ensure_base_stage`, `ensure_folder_acceptance_stage`, and `visible_result_from_sorted` still concentrated in one file.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline.rs` already has `parity_tests`, which implies sync/async semantic parity matters; the sync file itself has no local `#[cfg(test)]` block.
  - `docs/design_principles.md` explicitly prioritizes responsiveness and predictable interaction, both of which depend on stable browser-stage behavior.
- Recommended change: split the sync pipeline into explicit stage helpers for base refresh, folder-acceptance preparation, filtering/scoring, and final visible-row assembly, then add direct local tests around the sync stage boundaries rather than relying only on worker-side parity coverage.
- Expected impact: clearer browser-stage ownership and less risk that sync behavior drifts behind worker-side tests.
- Risks / tradeoffs: sync and async pipelines intentionally have different execution models; the refactor should share semantics where justified without forcing a broad merge.
- Dependencies: none
- Suggested validation:
  - targeted browser pipeline tests
  - targeted browser search-worker parity tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 6. Split `src/app_core/native_shell/map_projection.rs` into cache/query refresh, retained-point projection, and label assembly helpers
- Classification: Architecture improvement
- Confidence: Medium
- ROI: Medium-High
- Effort: M
- Why it matters: `project_map_model` is the main projection bridge for the map panel, but it currently mixes active-state gating, cache-key checks, DB/query refresh behavior, retained normalized-point caching, and user-facing label formatting in one long function.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `project_map_model` (`src/app_core/native_shell/map_projection.rs:11`) at 212 lines.
  - `src/app_core/native_shell/map_projection.rs:11`, `:223`, `:241`, `:259`, and `:283` show `project_map_model`, `map_projection_cache_key`, `refresh_projected_map_points_cache`, `build_projected_map_points_cache`, and `short_sample_label` in one file.
  - Tests exist in `src/app_core/native_shell/tests/map.rs`, but they validate higher-level map behavior rather than making the cache/query/label seams locally obvious.
  - `README.md` describes `src/app_core` as a stable projection/action bridge, which argues for keeping projection code explicit and easier to audit.
- Recommended change: extract focused helpers for inactive/early-return policy, retained-point cache refresh, DB/query fallback, and label assembly while preserving the projection surface and existing tests.
- Expected impact: better readability at the app-core/native-shell boundary and safer future work on map caching or labeling without broad function churn.
- Risks / tradeoffs: medium confidence because some projection coupling may be intentional for performance; the safest cut is helper extraction, not a layer move.
- Dependencies: none
- Suggested validation:
  - targeted `app_core::native_shell::tests::map`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 7. Separate waveform refresh scheduling/coalescing from actual render/apply logic in `src/app/controller/library/wavs/waveform_rendering.rs`
- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: waveform rendering has to stay responsive, but the current file still mixes refresh-reason coalescing, batch scheduling, render-meta reuse, decoded-image application, and immediate rerender behavior in one controller surface.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/library/wavs/waveform_rendering.rs` among the larger remaining controller hotspots.
  - `src/app/controller/library/wavs/waveform_rendering.rs:31`, `:70`, `:116`, `:201`, `:216`, and `:233` show reason merging, render-meta matching, batch begin/flush, and immediate refresh logic in one file.
  - Tests in `src/app/controller/tests/waveform_nav_render.rs` cover visible behavior and reuse thresholds, which indicates the behavior matters, but the scheduling/coalescing ownership boundary remains broad.
  - `docs/design_principles.md` emphasizes responsiveness and explicit ownership, both of which argue for a clearer split between “when to refresh” and “how to render/apply”.
- Recommended change: keep the current behavior but split the file into smaller helpers/modules for refresh scheduling/coalescing, render-meta reuse policy, and actual image generation/application.
- Expected impact: clearer ownership on a performance-sensitive controller path and easier targeted edits when waveform refresh rules change.
- Risks / tradeoffs: waveform reuse behavior is easy to perturb accidentally; validation needs to keep render-equivalence and loading-path tests in place.
- Dependencies: none
- Suggested validation:
  - targeted waveform render/reuse tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 8. Split `src/sample_sources/db/read.rs` into row-decoding, list/query families, and BPM lookup helpers with direct contract coverage
- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters: the read side of the source DB is smaller than the write side, but it still mixes path decoding, row decoding, several list/query shapes, and BPM lookup batching in one file. That makes it harder to reason about parity with the write surface and to add direct contract tests.
- Evidence:
  - `src/sample_sources/db/read.rs:11`, `:39`, `:179`, and `:221` show `decode_relative_path`, the main `impl SourceDatabase`, `list_files_page`, and `bpms_for_sample_ids` in one file.
  - `rg` finds no local `#[cfg(test)]` block in `src/sample_sources/db/read.rs`.
  - `README.md` documents per-source `.sempal_samples.db` files as a core persistence mechanism, which raises the value of explicit read/write contract clarity.
  - Existing tests are mostly higher-level call-site tests rather than direct read-surface contract tests for pagination, missing-path decoding, and batch BPM lookup behavior.
- Recommended change: extract row/path decoding helpers and query-family helpers into smaller modules, then add direct tests around pagination, missing-entry decoding, and batched BPM lookups.
- Expected impact: better discoverability of the read surface and stronger direct coverage of the source DB contract.
- Risks / tradeoffs: medium confidence because the current file is not the most urgent persistence hotspot; this should stay a focused cleanup after the write-surface work.
- Dependencies: item 1 should land first so read/write contract coverage evolves coherently
- Suggested validation:
  - targeted `sample_sources::db` read tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. How far should `vendor/radiant` native-shell/runtime decomposition go before it becomes speculative?
- Question: is the intended direction a few more focused helper/module splits, or a broader runtime/native-shell state redesign?
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_startup.rs`, `vendor/radiant/src/gui/native_shell/state/overlays.rs`, and `vendor/radiant/src/gui/native_shell/state/browser_rows/windowing.rs` are all still allowlisted oversized modules.
  - `vendor/radiant/src/gui/native_shell/mod.rs` remains very large, which suggests more structural debt exists but not necessarily the right next boundary.
- Why this matters: the right decomposition size changes depending on whether the repo wants local ownership splits or larger runtime/native-shell shape changes.
- Affected files/modules:
  - `vendor/radiant/src/gui_runtime/native_vello/*`
  - `vendor/radiant/src/gui/native_shell/*`
- Risk if guessed incorrectly: unnecessary churn in latency-sensitive GUI/runtime code.
- Most conservative provisional assumption: prefer local helper/module extraction that preserves current public runner/native-shell shapes.

### [!] 2. Should sync browser visible-row computation and async worker computation eventually converge further?
- Question: are the sync browser pipeline and async search-worker pipeline meant to stay separate long-term, or are they transitional implementations expected to share more pure logic?
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline.rs` still owns sync visible-row staging.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline.rs` already carries parity tests, which implies semantic equivalence matters.
- Why this matters: the safe cleanup boundary is different if the long-term plan is “shared semantics only” versus “eventual consolidation.”
- Affected files/modules:
  - `src/app/controller/library/wavs/browser_pipeline.rs`
  - `src/app/controller/library/wavs/browser_search_worker/pipeline.rs`
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/*`
- Risk if guessed incorrectly: either recurring parity drift or an over-ambitious merge that harms responsiveness/cancellation behavior.
- Most conservative provisional assumption: share only clearly duplicated pure stage semantics first; keep execution model, caching, and cancellation boundaries separate.

### [!] 3. What is the intended ownership boundary for source-DB contract helpers?
- Question: should `src/sample_sources/db/read.rs` and `write.rs` continue to be thin siblings under one `SourceDatabase` surface, or is there a longer-term push toward more explicit repository/transaction helper modules?
- Evidence:
  - `src/sample_sources/db/write.rs` and `src/sample_sources/db/read.rs` both mix helper types/functions with the main `impl SourceDatabase`.
  - The repo documents the per-source DB format and recovery behavior, but not the preferred internal organization of the `SourceDatabase` surface.
- Why this matters: the best refactor could be either targeted internal helper extraction or a more explicit internal repository split.
- Affected files/modules:
  - `src/sample_sources/db/read.rs`
  - `src/sample_sources/db/write.rs`
  - `src/sample_sources/db/mod.rs`
- Risk if guessed incorrectly: introducing structure that does not match the intended DB ownership model.
- Most conservative provisional assumption: keep the public `SourceDatabase` surface stable and improve internal helper boundaries plus direct tests first.

### [!] 4. How much map-panel policy should stay inside projection code versus controller/query code?
- Question: is `project_map_model` intentionally the place where cache/query policy and label assembly meet, or is that only an incidental concentration?
- Evidence:
  - `src/app_core/native_shell/map_projection.rs` currently owns both retained-point cache logic and user-facing summary/label assembly.
  - `README.md` and `docs/ARCHITECTURE.md` describe `src/app_core` as a projection bridge but do not define the preferred internal shape for map policy.
- Why this matters: helper extraction is safe either way, but moving behavior across layers would not be.
- Affected files/modules:
  - `src/app_core/native_shell/map_projection.rs`
  - `src/app_core/native_shell/tests/map.rs`
  - related map controller/query code under `src/app`
- Risk if guessed incorrectly: a cleanup could harden the wrong boundary and make later map work harder.
- Most conservative provisional assumption: keep map behavior in the same layer and only split helper responsibilities within the projection file.

## Rejected Ideas

### [-] 1. Split `vendor/radiant/src/app/actions.rs` by action family right now
- Why it was considered: it is still a large file and a discoverability hotspot.
- Why it was rejected: the file appears to act as an intentionally centralized action schema, and I did not find repository evidence that its current size is causing more immediate pain than the runtime/native-shell/browser/DB hubs above.
- What evidence was missing: no active plan note, test gap, or contradictory implementation showing that action-schema centralization is currently the highest-value problem.

### [-] 2. Reopen loop-crossfade flow work immediately
- Why it was considered: `src/app/controller/playback/loop_crossfade.rs` remains large and sits near file-modifying behavior.
- Why it was rejected: the current tree already contains direct controller tests at `src/app/controller/playback/loop_crossfade.rs:430` and `:451`, reducing its urgency relative to the source-DB surfaces that still lack local tests.
- What evidence was missing: no fresh failing coverage, TODO cluster, or repo note showing that loop-crossfade is still a top-priority gap after the recent coverage landed.

### [-] 3. Promote desktop AIV automation into default CI now
- Why it was considered: GUI automation remains an active repository direction.
- Why it was rejected: `docs/gui_test_platform.md` still treats the desktop AIV layer as stability-sensitive and broader than the normal quick loop.
- What evidence was missing: no current repository evidence that focus/foreground instability is solved well enough for default CI promotion.
