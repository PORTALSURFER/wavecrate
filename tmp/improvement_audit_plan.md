# Improvement Audit Plan

Generated: 2026-03-17
Observed commit: `5c69a9ce`
Status: Phase 1 complete on 2026-03-17. This file is the current ROI-ranked improvement backlog for the live tree and is waiting for explicit user confirmation before implementation.

## Scope

- This document supersedes the previous completed execution record that lived at this path.
- Items are ranked in strict execution order by expected ROI for the repository state observed on 2026-03-17.
- Recommendations stay inside repository-supported direction; speculative product expansion and preference-driven rewrites are excluded.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as a realtime-oriented Rust sample triage and curation tool for local audio libraries.
- Maturity level: Explicitly documented. `README.md` labels the app early alpha and warns that file operations can modify, rename, or delete user data.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace; `README.md` and `docs/ARCHITECTURE.md` document the vendored `radiant` runtime/UI layer.
- Repository shape: Explicitly documented. Domain/controller logic lives under `src/`; support tools live under `apps/` and `tools/`; GUI/runtime ownership is split between `src/app_core`, `src/gui*`, and `vendor/radiant`.
- Architectural boundaries: Explicitly documented. `docs/ARCHITECTURE.md` and `README.md` keep domain state and application logic in `src` while `vendor/radiant` owns GUI runtime behavior and host/runtime action wiring.
- Test strategy: Strongly implied by code/docs. `docs/TEST.md` and the source tree favor deterministic Rust unit/module tests plus targeted controller/integration coverage; GUI automation remains a broader local lane.
- Canonical local validation commands: Explicitly documented. `docs/README.md`, `docs/TEST.md`, and `AGENTS.md` define `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1` as the Windows validation ladder.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, non-blocking execution, reversibility, data integrity, and predictable interaction.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, or attention-retention product.

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for listening to, navigating, editing, and curating local sample libraries with strong emphasis on responsiveness and reversible workflows.
- What the repo appears to be moving toward: Strongly implied by code/docs. Safer staged file operations, stronger local guardrails, and tighter native-runtime/controller contracts with more explicit validation lanes.
- What is merely possible but unsupported: broad product-scope expansion, splitting intentionally centralized compatibility surfaces only because they are large, or reopening already-finished drag/drop integrity work without fresh contradictory evidence.

## Ordered Backlog

### 1. [ ] Refresh stale cleanup-hotspot and quality-score artifacts before using them for further prioritization

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: current planning inputs still describe an older tree and now mis-rank already-finished file-size debt, which makes follow-on cleanup prioritization less trustworthy.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` was generated at commit `9bda0d2e`, not the observed current commit `5c69a9ce`.
  - `tmp/cleanup_audit_hotspots.md` still reports 7 over-budget Rust files, including `src/app/controller/tests/drag_drop_drop_targets.rs` at 431 lines and `src/selection/range.rs` at 407 lines.
  - A fresh full scan on 2026-03-17 passed: `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` returned `[file_budget] OK`.
  - Current line counts on 2026-03-17 place `src/app/controller/tests/drag_drop_drop_targets.rs` at 390 lines, `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs` at 384, and `src/selection/range.rs` at 372.
  - `docs/QUALITY_SCORE.md` still tells maintainers to burn down the stale file-size list from `tmp/cleanup_audit_hotspots.md`.
- Recommended change: regenerate `tmp/cleanup_audit_hotspots.md`, update `docs/QUALITY_SCORE.md` to match the now-green full-scan file-size guardrail, and remove stale references to already-finished over-budget files.
- Expected impact: restores trustworthy debt-tracking inputs for later audits and cleanup work without changing runtime behavior.
- Risks / tradeoffs: this is meta-work only; the main risk is papering over a new hotspot if the refresh is done carelessly.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 2. [ ] Add direct controller coverage for folder-drop planning, rejection, and result-application paths

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the folder-drop controller seam shapes requests, enforces user-visible rejection rules, updates progress/status, and remaps UI state after worker completion, but the local tests focus on worker execution rather than these controller behaviors.
- Evidence:
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/plan.rs` contains `handle_samples_drop_to_folder` and `handle_folder_drop_to_folder`, including same-source checks, selected-source checks, root/self/descendant rejection, progress setup, and test-vs-runtime dispatch.
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/apply_result.rs` contains `apply_folder_sample_move_result` and `apply_folder_move_result`, which mutate wav-entry caches, folder state, manual folders, focus, and status text.
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs` embeds tests that call `run_folder_sample_move_task` and `run_folder_move_task`; the current local test module does not exercise the planner or apply-result entrypoints directly.
  - Repository search on 2026-03-17 found no direct test references to `handle_samples_drop_to_folder`, `handle_folder_drop_to_folder`, `apply_folder_sample_move_result`, or `apply_folder_move_result`.
- Recommended change: add focused controller tests for empty/sample-source mismatch rejection, selected-source mismatch rejection, root/self/descendant folder rejection, status/progress setup, cancelled/no-op result messaging, and UI remapping after successful folder moves.
- Expected impact: lowers regression risk in a user-facing file-operation controller boundary before any structural cleanup.
- Risks / tradeoffs: these tests need controller fixtures rather than isolated worker fixtures, so setup will be slightly heavier than the existing task-level tests.
- Dependencies: none
- Suggested validation:
  - targeted folder-move controller tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - user-run confirmation lane: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 3. [ ] Split the folder-move drag/drop module so orchestration code and worker tests stop sharing one hotspot file

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: one near-budget module currently acts as the public portal for planner/result code while also housing a large embedded worker-test suite, which weakens discoverability and keeps unrelated responsibilities coupled.
- Evidence:
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs` is 384 lines on 2026-03-17, close to the repository's 400-line hard limit.
  - The file's production responsibility is only three module declarations (`apply_result`, `plan`, `worker`), but the rest of the file is a large `#[cfg(test)] mod tests` block focused on worker-task behavior.
  - `AGENTS.md` and repository instructions call for small, focused files and hierarchical module decomposition over broad concatenated hotspots.
  - `tmp/cleanup_audit_hotspots.md` still names `folder_moves.rs` as an over-budget hotspot, which is now stale on exact count but still points at a real concentration seam.
- Recommended change: keep `folder_moves.rs` as a small module portal and move the embedded tests into focused sibling test modules, ideally aligned with `plan`, `apply_result`, and worker responsibilities.
- Expected impact: improves discoverability and keeps future folder-move changes from re-inflating one mixed-responsibility file.
- Risks / tradeoffs: test relocation changes file layout but not behavior; the main risk is over-fragmenting if the split is too fine-grained.
- Dependencies: item 2
- Suggested validation:
  - targeted folder-move tests
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 4. [ ] Add long-file parity coverage for the Symphonia fallback peak/analysis path

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: the Symphonia fallback still has its own long-file peak/analysis loop, but the current tests only prove a malformed WAV can decode, not that the fallback produces the same peak/analysis semantics as the shared waveform path.
- Evidence:
  - `src/waveform/decode/symphonia_reader.rs` contains `build_symphonia_peaks`, a 130-line peak/analysis builder for long files.
  - `src/waveform/peak_analysis.rs` now centralizes the same concepts for WAV decode and recording aggregation, but `symphonia_reader.rs` still uses its own loop.
  - The only local Symphonia test is `symphonia_fallback_decodes_ill_formed_riff_size` in `src/waveform/decode/symphonia_reader.rs`; there is no long-file mono/stereo parity coverage there.
  - `tmp/cleanup_audit_hotspots.md` lists `build_symphonia_peaks` as one of the largest remaining function spans in the repository.
- Recommended change: add characterization tests inside `symphonia_reader.rs` for long-file mono and stereo peak output, clamp behavior, analysis stride/sample-rate expectations, and parity against the shared helper semantics.
- Expected impact: creates a safety net around a duplicated decode path before any deduplication work.
- Risks / tradeoffs: the tests will need deterministic fixtures that force the long-file fallback path rather than the short fully-decoded path.
- Dependencies: none
- Suggested validation:
  - targeted waveform decode tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - user-run confirmation lane: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 5. [ ] Route Symphonia long-file peak/analysis building through the shared `waveform::peak_analysis` helpers

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: after the recent recording/WAV cleanup, the Symphonia fallback is now the remaining visible duplicate owner of the peak-bucket, clamp, and analysis-accumulation logic.
- Evidence:
  - `src/waveform/decode/symphonia_reader.rs` manually computes `bucket_size_frames`, `analysis_stride`, mono/left/right peak ranges, and analysis accumulation in `build_symphonia_peaks`.
  - `src/waveform/decode/peaks.rs` already re-exports shared `analysis_stride` and `peak_bucket_size` from `src/waveform/peak_analysis.rs`.
  - `src/app/controller/playback/recording/waveform_loader/aggregation.rs` was already moved onto `PeakAnalysisAccumulator`, leaving `symphonia_reader.rs` as the obvious remaining drift seam.
- Recommended change: reuse `PeakAnalysisAccumulator` or a narrow shared helper from `src/waveform/peak_analysis.rs` so Symphonia long-file fallback no longer owns a second copy of the same math.
- Expected impact: reduces future correctness drift and keeps waveform math changes localized.
- Risks / tradeoffs: the helper boundary should stay internal and small; forcing an over-general abstraction would hurt readability more than it helps.
- Dependencies: item 4
- Suggested validation:
  - the new Symphonia parity tests from item 4
  - targeted waveform decode tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 6. [ ] Add direct coverage for audio-load message routing and transient cache-token gating

- Classification: Test gap
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: one controller boundary decides whether background audio results still belong to the active request and whether transient markers should be persisted or ignored, but current tests only cover adjacent end-to-end effects rather than these explicit routing branches.
- Evidence:
  - `src/app/controller/library/background_jobs/polling/audio.rs` contains `handle_audio_loaded_message`, which matches `AudioLoadResult::Primary` against pending request identity, clears pending/loading state, and routes `AudioLoadResult::Transients` into transient handling.
  - `src/app/controller/library/wavs/audio_loading.rs` contains `handle_audio_transients_loaded`, which rejects results on source/path mismatch or `cache_token` mismatch and persists cache only for unstretched results.
  - Repository search on 2026-03-17 found no direct test references to `handle_audio_loaded_message`, `AudioLoadResult::Primary`, `AudioLoadResult::Transients`, or `handle_audio_transients_loaded`.
  - Existing tests in `src/app/controller/tests/waveform_nav_render.rs` and `src/app/controller/tests/waveform_cache_loading.rs` cover stale end-to-end audio results and cache-hit reuse, but not these branch conditions directly.
- Recommended change: add focused controller tests for primary-result mismatch ignore, pending/loading clear-on-match behavior, transient source/path/token mismatch ignore paths, and unstretched transient persistence.
- Expected impact: makes a fragile request-routing seam cheaper to change safely and complements the broader audio-loader tests already in place.
- Risks / tradeoffs: some tests may need local helper construction for `AudioLoadResult` and decoded waveform state rather than full async job execution.
- Dependencies: none
- Suggested validation:
  - targeted controller audio tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - user-run confirmation lane: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. Should `vendor/radiant/src/app/actions/mod.rs` remain one intentionally centralized compatibility surface?

- Evidence:
  - The module docs in `vendor/radiant/src/app/actions/mod.rs` explicitly say `UiAction` intentionally remains the single compatibility surface between the native runtime and the host bridge.
  - The same docs say the preferred maintenance approach is to keep the enum centralized while improving internal organization around it.
- Why this matters: future cleanup passes will keep nominating the file by size unless the intentional contract boundary stays explicit.
- Affected files/modules: `vendor/radiant/src/app/actions/mod.rs`, runtime action routing, host bridge, automation catalog.
- Risk if guessed incorrectly: premature splitting could destabilize the one inspectable action surface shared across runtime, host, and automation code.
- Most conservative provisional assumption: keep `UiAction` centralized and only revisit that choice if a concrete routing or ownership mismatch appears.

### [!] 2. Should `src/selection/range.rs` continue to keep waveform geometry, fades, and gain math together?

- Evidence:
  - The module docs in `src/selection/range.rs` explicitly describe the file as one cohesive waveform-editing domain model.
  - The file is still large enough to attract size-driven cleanup suggestions even though the current docs argue for cohesion.
- Why this matters: size-only cleanup pressure could split a correctness-sensitive domain contract without clear behavioral benefit.
- Affected files/modules: `src/selection/range.rs`, waveform selection preview, fade handles, destructive edit flows.
- Risk if guessed incorrectly: over-splitting could scatter one normalized selection/fade contract across several low-value helpers and make invariants harder to reason about.
- Most conservative provisional assumption: keep the module cohesive unless a clearer subdomain or ownership boundary emerges.

## Rejected Ideas

### [-] 1. Reopen the cross-source drop-target integrity lane

- Why it was considered: previous audits found a real journal/recovery gap in the cross-source drop-target path.
- Why it was rejected: the current tree already stages drop-target copy/move work through `move_transaction` helpers, updates `file_ops_journal` stages, and has direct regression coverage in `src/app/controller/tests/drag_drop_drop_targets.rs`.
- What evidence was missing: any live mismatch between the documented journal contract and the current implementation.

### [-] 2. Split `vendor/radiant/src/app/actions/mod.rs` immediately

- Why it was considered: it remains one of the largest live Rust modules by raw line count.
- Why it was rejected: the file itself explicitly documents that the centralized action catalog is intentional and shared by runtime, host bridge, and automation code.
- What evidence was missing: any concrete routing, ownership, or regression pain caused by the current centralized enum.

### [-] 3. Split `src/selection/range.rs` immediately

- Why it was considered: it is dense, mathematically non-trivial, and still relatively large.
- Why it was rejected: the file itself now documents that geometry, fades, and gain rules are one shared waveform-editing domain model.
- What evidence was missing: any demonstrated subdomain boundary that would make a split safer or clearer.
