# Improvement Audit Plan

Generated: 2026-03-18
Observed commit: `14dfd479`
Status: Phase 2 implementation completed on 2026-03-19. This file remains the live execution record for the current tree.

## Scope

- This document supersedes the previous plan snapshot that lived at this path.
- Items are ranked in strict execution order by expected ROI for the repository state observed on 2026-03-18.
- Recommendations stay inside repository-supported direction; speculative product expansion and preference-driven rewrites are excluded.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as a realtime-oriented Rust sample triage and curation tool for local audio libraries.
- Maturity level: Explicitly documented. `README.md` labels the app early alpha and warns that file operations can modify, rename, or delete user data.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace with the vendored `radiant` runtime/UI layer plus helper apps and tools under `apps/` and `tools/`.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` splits domain/controller logic under `src/`, runtime/UI framework code under `vendor/radiant/`, and helper binaries under `apps/` / `tools/`.
- Architectural boundaries: Explicitly documented. `README.md` and `docs/ARCHITECTURE.md` keep domain state and UI intent in `src` while `vendor/radiant` owns runtime UI behavior.
- Test strategy: Strongly implied by code/docs. `docs/TEST.md` and the source tree favor deterministic Rust unit/module tests, targeted controller tests, and broader GUI contract/AIV lanes outside the default constrained-agent loop.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, non-blocking execution, reversibility, data integrity, and predictable interaction.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, or attention-retention product.

## Audit Notes

- Full file-size guardrail is currently degraded on the live tree: `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` currently reports `src/app/controller/tests/drag_drop_drop_targets.rs:477` as the remaining over-budget file outside the folder-move split item.
- High-visibility score drift currently passes with degraded guardrails: `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` now reports score `3` because active file-size-budget and Rust-taste violations still exist outside this backlog slice.
- The broader agent-safe validation lane now completes in this environment after the Windows PowerShell wrappers added an unhealthy-`sccache` fallback plus a repo-local temp-dir fallback for constrained sessions.

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for listening to, navigating, editing, and curating local sample libraries with strong emphasis on responsiveness and reversible workflows.
- What the repo appears to be moving toward: Strongly implied by code/docs. More explicit background-job boundaries for file operations, tighter controller/runtime contracts, and stronger local validation/guardrail hygiene.
- What is merely possible but unsupported: sweeping GUI/catalog rewrites, splitting intentionally centralized compatibility surfaces only because they are large, or reopening already-covered recovery systems without fresh contradictory evidence.

## Ordered Backlog

### 1. [x] Move cross-source drop-target copy/move work off the controller thread and onto the existing file-op worker pipeline

- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: this path performs filesystem staging, SQLite writes, and final file moves inline on the controller thread even though the documented design forbids blocking file I/O and DB work on the UI path.
- Evidence:
  - `docs/design_principles.md` says blocking file I/O, analysis, indexing, rendering, and database access are explicitly prohibited on the UI thread.
  - `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs:37` runs `prepare_staged_move`, target/source DB mutations, and `move_sample_file` inline inside `handle_sample_drop_to_drop_target`.
  - `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs:258` runs `prepare_staged_copy`, DB writes, and `move_sample_file` inline inside `copy_sample_to_target`.
  - `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs:237` handles multi-sample drops by looping the same synchronous single-sample path.
  - Equivalent file-op flows already use background jobs plus progress/cancellation: `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/plan.rs` and `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/plan.rs`.
- Recommended change: route cross-source drop-target moves/copies through the same `FileOpMessage`/worker-thread pipeline used by folder/source moves, with progress visibility and cancellation support.
- Expected impact: removes a UI-thread blocking path, aligns drop-target behavior with the repo’s non-blocking contract, and reduces hitch risk on large or slow filesystem moves.
- Risks / tradeoffs: workerizing the path will require careful result-application wiring so existing cache/journal semantics remain intact.
- Dependencies: none
- Suggested validation:
  - targeted drag/drop controller tests for cross-source drop targets
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - user-run confirmation lane: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: 2026-03-19
- Commit: `7e6baff1` (`fix(drag-drop): workerize drop target transfers`)
- Assumptions used:
  - cache-backed wav metadata remains the authoritative source for dragged samples when it is available locally, with source DB reads used only as worker-side fallback
  - same-source non-copy drop-target batches should keep delegating to the existing folder-move pipeline when every sample already belongs to the target source
- Validation outcome:
  - `cargo test drag_drop_drop_targets -- --nocapture` passed when launched through PowerShell 7 with `RUSTC_WRAPPER` cleared and `TEMP`/`TMP` pointed at `tmp/agent_temp`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` still failed before meaningful validation because Cargo invoked the pre-existing unhealthy `sccache` wrapper path tracked separately in backlog item 4
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` failed for the same pre-existing `sccache` wrapper reason

### 2. [x] Add direct controller coverage for folder-move planning and result-application branches

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the folder-move controller layer owns user-visible rejection rules, progress setup, remapping/focus updates, and status text, but current local coverage concentrates on worker behavior instead of these controller-only branches.
- Evidence:
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/plan.rs:29` and `:173` contain source validation, same-folder rejection, root/self/descendant folder rejection, progress setup, and test-vs-runtime dispatch.
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/apply_result.rs:9` and `:82` apply moved-entry cache updates, remap folders, update focus, and synthesize status messages.
  - Repo-wide search found no direct tests for `handle_samples_drop_to_folder`, `handle_folder_drop_to_folder`, `apply_folder_sample_move_result`, or `apply_folder_move_result`.
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs` currently tests `run_folder_sample_move_task` and `run_folder_move_task`, which covers worker execution rather than controller branches.
- Recommended change: add focused controller tests for source mismatch rejection, selected-source mismatch rejection, root/self/descendant rejection, progress overlay setup, cancelled/no-op status messaging, and folder-state/focus remapping after success.
- Expected impact: makes a user-facing file-op boundary safer to change and protects controller semantics that worker tests cannot catch.
- Risks / tradeoffs: these tests will need controller fixtures rather than isolated worker fixtures, so setup will be slightly heavier.
- Dependencies: none
- Suggested validation:
  - targeted folder-move controller tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-19
- Commit: `c6b814d2` (`test(controller): cover folder move branches`)
- Assumptions used:
  - direct controller tests in `src/app/controller/tests/drag_drop_folders.rs` are the lowest-risk way to cover the planning and apply-result seams without broadening production scope
- Validation outcome:
  - `cargo test drag_drop_folders -- --nocapture` passed when launched through PowerShell 7 with `RUSTC_WRAPPER` cleared and `TEMP`/`TMP` pointed at `tmp/agent_temp`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` still failed before meaningful validation because Cargo invoked the pre-existing unhealthy `sccache` wrapper path tracked separately in backlog item 4
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` failed for the same pre-existing `sccache` wrapper reason

### 3. [x] Add direct regression coverage for cross-source move result application and touched-source invalidation

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: S-M
- Why it matters: source-move result application mutates caches across multiple sources and synthesizes user-facing status, but the current tests focus on request collection and worker transactions rather than the controller result seam.
- Evidence:
  - `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/apply_result.rs:9` applies move results, invalidates touched sources, and translates outcomes into status text.
  - Repo-wide search found no direct tests for `apply_source_move_result`, `apply_moved_source_entries`, `invalidate_moved_sources`, or `set_source_move_status`.
  - Existing tests in `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/plan.rs` and `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/worker.rs` cover request/worker behavior instead.
- Recommended change: add focused controller tests for multi-source invalidation, partial-success status text, cancelled/no-op messaging, and target-source cache refresh behavior.
- Expected impact: reduces risk of stale browser/source state after cross-source moves and complements the worker-level transaction safety already in place.
- Risks / tradeoffs: low implementation risk; the main tradeoff is extra fixture setup for multi-source controller state.
- Dependencies: none
- Suggested validation:
  - targeted source-move controller tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-19
- Commit: `91e5c30e` (`test(controller): cover source move apply-result branches`)
- Assumptions used:
  - direct `apply_source_move_result` tests need to seed the source and target databases to match the background worker contract before selected-source invalidation reloads from disk
  - when the target source is currently selected, touched-source invalidation should clear stale state and repopulate the visible wav list from the refreshed target DB rather than leave the selected cache empty
- Validation outcome:
  - `cargo fmt --all` passed
  - `cargo test drag_drop_sources -- --nocapture` passed with `RUSTC_WRAPPER` cleared and `TEMP`/`TMP` pointed at `tmp/agent_temp`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` still failed before meaningful validation because the documented wrapper kept the pre-existing unhealthy `sccache` path active
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` failed for the same pre-existing `sccache` wrapper reason

### 4. [x] Make the documented Windows agent-safe validation lane resilient when `sccache` is installed but unhealthy

- Classification: Developer-experience improvement
- Confidence: Medium
- ROI: High
- Effort: S-M
- Why it matters: the repository’s documented constrained-environment validation lane is supposed to be the safe default for agent work on Windows, but in the current environment it failed before meaningful validation because `sccache` was treated as usable just because it was installed.
- Evidence:
  - `scripts/use_cargo_cache.ps1` sets `RUSTC_WRAPPER` to `sccache` whenever `sccache` is found and `SEMPAL_DISABLE_SCCACHE` is not set.
  - `scripts/devcheck.ps1` and `scripts/ci_agent.ps1` always call `Enable-SempalCargoCache`.
  - On 2026-03-18, `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` failed during `cargo check -p sempal --tests --bins` because `embed-resource` could not query `rustc` through a timed-out `sccache` startup.
  - `docs/README.md`, `docs/TEST.md`, and `AGENTS.md` all present `scripts/ci_agent.ps1` as the constrained-environment validation lane.
- Recommended change: make the cache helper degrade gracefully when `sccache` is installed but unhealthy, for example by probing it, falling back to direct `rustc`, or retrying without `RUSTC_WRAPPER` when the wrapper itself is the failure source.
- Expected impact: restores the reliability of the documented Windows agent validation path in the environments it is meant to support.
- Risks / tradeoffs: cache probing/fallback logic adds script complexity and should avoid masking genuine compiler failures.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - targeted script guardrails if the helper behavior changes
- Product clarification required: No
- Completed: 2026-03-19
- Commit: `d0685aad` (`fix(scripts): harden windows cargo wrapper fallback`)
- Assumptions used:
  - the documented PowerShell validation lane should prefer direct `rustc` over an inherited or globally configured `sccache` wrapper when the wrapper cannot successfully answer `rustc --version`
  - falling back to `tmp/agent_temp` is an acceptable constrained-environment prerequisite because the repo already uses workspace-local temp paths for successful targeted Windows cargo runs in this environment
  - passing Cargo a direct `--config build.rustc-wrapper=''` override is safer than editing user-global Cargo config and is narrow enough for repo-owned wrapper scripts
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1` passed

### 5. [x] Enforce that `DesktopAiv` catalog coverage claims are backed by actual desktop-AIV cases

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the GUI action catalog advertises required coverage layers per action, but current tests only verify that coverage metadata exists, not that the promised desktop-AIV coverage is actually present.
- Evidence:
  - `src/app_core/actions/catalog/entries.rs` marks several actions with `DesktopAiv` coverage, including `focus_browser_search`, `open_options_menu`, `select_source_row`, `move_browser_focus`, `focus_browser_row`, `seek_waveform`, `set_waveform_cursor`, and `set_waveform_view_center`.
  - `src/app_core/actions/tests.rs` checks catalog completeness, unique IDs, and non-empty coverage layers, but it does not verify pack/case coverage against those declarations.
  - The current desktop-AIV manifests are assembled in `src/gui_test/aiv/packs.rs` and case files under `src/gui_test/aiv/packs/cases/`.
  - Repo-wide search in the desktop-AIV case files found no coverage for several catalog-marked `DesktopAiv` action IDs, while the metadata still claims that coverage exists.
- Recommended change: add a coverage-consistency test or generator check that compares catalog `DesktopAiv` claims against the exported AIV suite manifests, then either add the missing cases or downgrade the metadata until it is true.
- Expected impact: makes GUI coverage metadata trustworthy instead of aspirational and reduces the chance of over-reporting desktop automation coverage.
- Risks / tradeoffs: if the catalog intentionally models future coverage, this will force the repo to choose between building those cases now or explicitly narrowing the claims.
- Dependencies: none
- Suggested validation:
  - targeted GUI action catalog tests
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - user-run confirmation lane: `powershell -ExecutionPolicy Bypass -File scripts/run_gui_suite.ps1`
- Product clarification required: No
- Completed: 2026-03-19
- Commit: `8b0637d7` (`test(gui): align desktop aiv coverage claims`)
- Assumptions used:
  - when catalog metadata claims `DesktopAiv` coverage, the exported named desktop-AIV manifests should contain at least one `AssertActionRecorded` assertion for that action id
  - downgrading unsupported `DesktopAiv` claims is safer than inventing new desktop cases mid-backlog because the current manifests are the stronger source of truth for what is actually exercised today
- Validation outcome:
  - `cargo fmt --all` passed
  - `cargo test desktop_aiv_catalog_claims_are_backed_by_manifest_cases -- --nocapture` passed with direct-`rustc` and repo-local temp overrides
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` did not complete in this host because the wrapper hit a pre-existing language-mode dot-sourcing restriction before it could run the contract suite

### 6. [x] Add controller-branch tests for `AudioLoadResult` routing and transient cache-token gating

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: one controller seam decides whether late audio results still match the active request, whether waveform loading state is cleared, and whether transient data should be applied or persisted.
- Evidence:
  - `src/app/controller/library/background_jobs/polling/audio.rs:50` routes `AudioLoadResult::Primary` through pending-request matching, clears pending/loading state, and dispatches success/error handling.
  - `src/app/controller/library/wavs/audio_loading.rs:126` rejects transient updates on source/path mismatch or `cache_token` mismatch and only persists cache for unstretched results.
  - `src/app/controller/library/background_jobs/polling/tests.rs` exercises folder-scan, search, and generic file-op progress handling but not audio-loaded message branches.
  - `src/app/controller/tests/waveform_cache_loading.rs` covers cache reuse and reload behavior, but repo-wide search found no direct tests for `handle_audio_loaded_message` or `handle_audio_transients_loaded`.
- Recommended change: add focused tests for stale primary results, matching primary-result cleanup of pending/loading state, transient source/path/token mismatch ignores, and unstretched-only transient persistence.
- Expected impact: lowers regression risk for late worker results attaching to the wrong waveform or incorrectly mutating cache state.
- Risks / tradeoffs: these tests need deliberate setup of pending audio state and decoded waveform tokens.
- Dependencies: none
- Suggested validation:
  - targeted controller audio-routing tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-19
- Commit: `78430bfa` (`test(controller): cover audio load routing branches`)
- Assumptions used:
  - routing coverage for the `AudioLoadResult::Transients` controller branch can safely use a stretched transient payload because the router contract is only dispatch, while cache-persistence behavior is covered separately in direct transient-gating tests
  - transient cache-token gating is best locked down at the `handle_audio_transients_loaded` seam because that method owns the source/path/token guards and the stretched-vs-unstretched cache mutation split
- Validation outcome:
  - `cargo fmt --all` passed
  - `cargo test audio_primary_message_ignores_stale_completion_then_applies_matching_result -- --nocapture` passed with direct-`rustc` and repo-local temp overrides
  - `cargo test audio_transients_message_routes_to_loaded_waveform_state -- --nocapture` passed with direct-`rustc` and repo-local temp overrides
  - `cargo test transient_results_require_matching_loaded_source_path_and_cache_token -- --nocapture` passed with direct-`rustc` and repo-local temp overrides
  - `cargo test transient_results_update_cache_only_for_non_stretched_waveforms -- --nocapture` passed with direct-`rustc` and repo-local temp overrides
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed

### 7. [x] Add long-file parity coverage for the Symphonia fallback peak/analysis path

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: the tolerant Symphonia fallback still owns its own long-file peak/analysis loop, but current tests only prove malformed-WAV decode success rather than parity of the long-file output semantics.
- Evidence:
  - `src/waveform/decode/symphonia_reader.rs:64` implements `build_symphonia_peaks` for long files.
  - The only local test in that file is `symphonia_fallback_decodes_ill_formed_riff_size` at `src/waveform/decode/symphonia_reader.rs:213`.
  - `src/waveform/decode/cache_token.rs:47` exposes `load_decoded_with_max_frames`, which can force the long-file branch for deterministic tests.
  - `src/waveform/decode/peaks.rs` and `src/waveform/peak_analysis.rs` define the shared long-waveform semantics that the Symphonia path should match, but there is no direct parity coverage.
- Recommended change: add characterization tests for mono/stereo long-file fallback output covering total frames, bucket sizing, left/right/mono peaks, analysis stride, and analysis sample-rate behavior.
- Expected impact: creates a safety net around the less-common tolerant decode path before any deduplication work.
- Risks / tradeoffs: exact bucket-for-bucket equality may be more brittle than invariant-based parity, so the test oracle should be chosen carefully.
- Dependencies: none
- Suggested validation:
  - targeted waveform decode tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-19
- Commit: `6b24829d` (`test(waveform): cover symphonia long-file parity`)
- Assumptions used:
  - the clean WAV long-file branch is the strongest repository-local oracle for Symphonia fallback parity because it already routes through the shared `PeakAnalysisAccumulator`
  - forcing `max_frames = 1` is an acceptable deterministic seam for both the clean WAV path and the malformed-byte Symphonia fallback path because `load_decoded_with_max_frames` exists explicitly as a test hook
- Validation outcome:
  - `cargo fmt --all` passed
  - `cargo test symphonia_reader::tests -- --nocapture` initially failed because the new parity tests exposed a real trailing-sentinel peak-bucket mismatch in the Symphonia EOF path
  - `cargo test symphonia_reader::tests -- --nocapture` passed after tightening EOF truncation to the actual used bucket count
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Deviation from original plan order:
  - item 7 required one tiny prerequisite production fix in `src/waveform/decode/symphonia_reader.rs` so the new parity coverage could pass against the repository's shared long-file semantics; this stayed within the current item because it was directly exposed by the added tests and did not broaden into the item 8 refactor

### 8. [x] Route Symphonia long-file peak/analysis building through the shared `PeakAnalysisAccumulator`

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: the Symphonia fallback is now the visible remaining duplicate owner of long-waveform bucket sizing, clamping, and analysis-sample accumulation math.
- Evidence:
  - `src/waveform/decode/symphonia_reader.rs:75-185` manually computes bucket sizing, mono/left/right peaks, clamping, and averaged analysis samples.
  - `src/waveform/peak_analysis.rs:40-151` already centralizes those concerns in `PeakAnalysisAccumulator`.
  - `src/waveform/decode/peaks.rs:63-97` already uses the shared accumulator for the primary WAV long-file path.
- Recommended change: after adding parity coverage, reuse `PeakAnalysisAccumulator` or a narrow helper built on it so the Symphonia fallback no longer owns a second copy of the same waveform math.
- Expected impact: reduces future correctness drift and keeps long-waveform math changes localized to one implementation.
- Risks / tradeoffs: the abstraction boundary should stay narrow; over-generalizing the helper would harm readability.
- Dependencies: item 7
- Suggested validation:
  - the new Symphonia parity tests from item 7
  - targeted waveform decode tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-19
- Commit: `6a8c78bd` (`refactor(waveform): share symphonia peak accumulation`)
- Assumptions used:
  - `PeakAnalysisAccumulator::output()` should represent the frames actually accumulated rather than the initial estimate used to size its buffers, because estimate-based callers like the Symphonia fallback can legitimately decode fewer frames than anticipated
  - keeping the Symphonia EOF shape identical to the now-landed item 7 parity tests is the right guardrail for this refactor, even though the helper tightening also benefits future estimate-based callers more generally
- Validation outcome:
  - `cargo fmt --all` passed
  - `cargo test peak_analysis::tests -- --nocapture` initially failed because `src/waveform/decode/peaks.rs` still re-exported sizing helpers that the refactor no longer used; the dead re-export was removed and the test then passed
  - `cargo test symphonia_reader::tests -- --nocapture` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed

### 9. [x] Refresh stale architecture and planning docs that no longer match the live tree

- Classification: Documentation gap
- Confidence: High
- ROI: Medium
- Effort: S-M
- Why it matters: several high-visibility handoff and architecture docs still point maintainers toward modules or debt states that no longer exist, which increases the risk of misrouting future work.
- Evidence:
  - `README.md` still references `src/legacy_runtime` and `src/gui_app`, but those paths do not exist in the current `src/` tree.
  - `docs/ARCHITECTURE.md` still routes “app-level GUI wiring” to `src/gui_app` and describes `src/gui` as app-level GUI wiring, while `src/gui/mod.rs` is now only `radiant` re-exports.
  - `docs/INDEX.md` still describes checks in terms of `crate::legacy_runtime::` and `crate::gui_app::` dependencies.
  - `tmp/cleanup_audit_hotspots.md` was generated at commit `9bda0d2e` and still reports seven over-budget files even though `scripts/check_file_size_budget.ps1 --all` now passes on commit `14dfd479`.
  - `docs/QUALITY_SCORE.md` still points maintainers at the stale cleanup-hotspot file-size debt list.
- Recommended change: update the architecture/handoff docs to reflect the live module map, regenerate `tmp/cleanup_audit_hotspots.md`, and refresh `docs/QUALITY_SCORE.md` so planning notes match the current guardrail state.
- Expected impact: keeps future contributors and agents aligned with the live tree instead of historical module names and stale debt lists.
- Risks / tradeoffs: documentation-only work can still hide omissions if the refresh is done without checking the live source tree carefully.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
- Product clarification required: No
- Completed: 2026-03-19
- Commit: `ac746e8e` (`docs: refresh architecture and audit snapshots`)
- Assumptions used:
  - item 9 should describe the live guardrail state at implementation time, even when it has drifted since the earlier audit snapshot; preserving a stale “healthy” story would be less accurate than updating the docs to match the current tree
  - boundary docs should distinguish between live directories (`src/gui_runtime`, `src/app`) and historical dependency tokens that the guardrail scripts still check for compatibility reasons (`crate::gui_app::`, `crate::legacy_runtime::`)
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` passed and regenerated `tmp/cleanup_audit_hotspots.md`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1` passed
  - `cmd /c pwsh -NoProfile -File scripts/check_quality_score_drift.ps1` passed; the script now reports a degraded score of `3` for `Agent-facing guardrails` because the current tree has active file-size-budget and Rust-taste violations outside this docs-only slice
- Deviation from original plan order:
  - item 9 originally assumed the full file-size guardrail was still green, but the live tree had regressed by implementation time; the docs were updated to match current reality rather than the older all-clear snapshot

### 10. [x] Split `folder_moves.rs` so the module portal and worker-heavy tests stop sharing one hotspot file

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-Low
- Effort: S
- Why it matters: the public folder-move module is close to the file-size limit even though most of its content is a single embedded worker-test block rather than production orchestration code.
- Evidence:
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs` is 384 lines on the live tree.
  - Its production responsibility is only the three submodule declarations (`apply_result`, `plan`, `worker`), while most of the file is `#[cfg(test)] mod tests`.
  - `AGENTS.md` and repo guidance call for small, focused files and hierarchical decomposition as files approach the 400-line budget.
- Recommended change: keep `folder_moves.rs` as a small module portal and move the embedded tests into focused sibling modules aligned with worker/planner responsibilities.
- Expected impact: improves discoverability and reduces the chance that future folder-move work keeps piling into the same near-budget file.
- Risks / tradeoffs: this is structural churn only; the main risk is splitting the tests too finely and making them harder to navigate.
- Dependencies: item 2
- Suggested validation:
  - targeted folder-move tests
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Product clarification required: No
- Completed: 2026-03-19
- Commit: `4485f18e` (`refactor(drag-drop): split folder move worker tests`)
- Assumptions used:
  - moving the worker-heavy tests under `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/tests/` is the smallest repository-aligned split because the production `folder_moves.rs` file only needs to stay a documented module portal
  - serializing the folder-move worker tests behind one local lock is an acceptable test-only safeguard because the worker exposes global one-shot hooks used to force deterministic DB timing
- Validation outcome:
  - `cargo fmt --all` passed
  - `cargo test folder_sample_move_ -- --nocapture` passed with direct-`rustc` and repo-local temp overrides
  - `cargo test folder_move_ -- --nocapture` passed with direct-`rustc` and repo-local temp overrides
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` still fails on the pre-existing unrelated violation `src/app/controller/tests/drag_drop_drop_targets.rs:477`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed

## Open Questions / Missing Definitions

### [!] 1. Was the synchronous cross-source drop-target path intentional, or is it an unfinished exception to the non-blocking file-op model?

- Evidence:
  - `docs/design_principles.md` forbids blocking file I/O and DB work on the UI thread.
  - `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs` still performs staged file work inline.
  - Parallel file-op flows in `folder_moves` and `source_moves` already use background workers plus progress.
- Why this matters: item 1 is a stronger recommendation if the current synchronous path is accidental drift rather than an intentional UX carveout.
- Affected files/modules: `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs`, file-op job routing, drag/drop progress UI.
- Risk if guessed incorrectly: changing execution mode could subtly affect drag/drop timing or status semantics if some caller depends on synchronous completion.
- Most conservative provisional assumption: treat this as unintended drift and align it with the documented non-blocking file-op model unless maintainers document a specific exception.

### [!] 2. Should `vendor/radiant/src/app/actions/mod.rs` remain one centralized compatibility surface?

- Evidence:
  - The module docs in `vendor/radiant/src/app/actions/mod.rs` explicitly say `UiAction` intentionally remains the single compatibility surface between the native runtime and the host bridge.
  - The same docs state the preferred maintenance approach is to keep the enum centralized while improving internal organization around it.
- Why this matters: large-file heuristics will keep nominating this module unless the intentional centralization remains explicit in the audit record.
- Affected files/modules: `vendor/radiant/src/app/actions/mod.rs`, app-core action catalog, runtime/automation bridges.
- Risk if guessed incorrectly: premature splitting could destabilize the one inspectable action surface shared across runtime, host, and automation code.
- Most conservative provisional assumption: keep `UiAction` centralized unless a concrete routing or ownership defect appears.

### [!] 3. Should `src/selection/range.rs` continue to keep waveform geometry, fades, and gain math together?

- Evidence:
  - The module docs in `src/selection/range.rs:1-7` explicitly describe the file as one cohesive waveform-editing domain model.
  - `src/selection/tests.rs` provides broad existing coverage across selection, fade, mute, snapping, and gain semantics.
- Why this matters: file-size heuristics still make this module look like cleanup debt even though the current docs and tests argue for cohesion.
- Affected files/modules: `src/selection/range.rs`, waveform edit-selection logic, fade preview, destructive edit flows.
- Risk if guessed incorrectly: over-splitting could scatter one correctness-sensitive contract across low-value helper modules.
- Most conservative provisional assumption: keep the module cohesive unless a clearer subdomain or ownership boundary emerges.

## Rejected Ideas

### [-] 1. Split `src/selection/range.rs` immediately

- Why it was considered: it remains a large, mathematically dense file.
- Why it was rejected: the file explicitly documents why the range/fade/gain rules stay together, and the existing tests already cover the domain broadly.
- What evidence was missing: a concrete ownership mismatch, correctness bug, or repeated change friction signal.

### [-] 2. Split `vendor/radiant/src/app/actions/mod.rs` immediately

- Why it was considered: it is still one of the broader live Rust modules by raw size.
- Why it was rejected: the module itself documents that the centralized action surface is intentional for runtime/host/automation compatibility.
- What evidence was missing: any concrete routing or maintenance failure caused by that centralization.

### [-] 3. Redesign browser-preview audio loading

- Why it was considered: preview audio loading is a specialized path with separate behavior from full selection loading.
- Why it was rejected: `src/app/controller/tests/browser_actions/focus_navigation.rs` already covers the preview queueing semantics, and this audit found no concrete defect or contract mismatch there.
- What evidence was missing: any documented bug, contradictory test, or stale implementation signal.

### [-] 4. Reopen file-op journal or folder-delete recovery as the next primary concern

- Why it was considered: these are correctness-sensitive recovery systems.
- Why it was rejected: the implementation and tests currently line up with their docs (`src/sample_sources/db/file_ops_journal/tests.rs` and `src/app/controller/tests/folders_core/rename_delete_recovery.rs`), and this pass found no fresh contradictory evidence.
- What evidence was missing: a live mismatch between the recovery contracts and the current code.
