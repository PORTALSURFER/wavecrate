# Improvement Audit Plan

Generated: 2026-03-17
Status: Phase 2 in progress on 2026-03-17. Items 1-3 are complete; items 4-6 remain in ranked order.

## Scope

- This document is the current evidence-driven improvement audit for the live tree.
- It supersedes the completed execution record that previously lived at this path.
- Items are ranked in strict execution order by expected ROI for the repository state observed on 2026-03-17.
- Recommendations stay inside repository-supported direction; speculative product or platform expansion is excluded.

## Repository Context

- Project purpose: Explicitly documented. [README.md](/C:/dev/sempal/README.md) and [docs/design_principles.md](/C:/dev/sempal/docs/design_principles.md) describe Sempal as a realtime-oriented Rust sample triage and curation tool for local audio libraries.
- Maturity level: Explicitly documented. [README.md](/C:/dev/sempal/README.md) labels the app early alpha and warns that file operations can modify, rename, or delete user data.
- Primary languages / frameworks / tooling: Explicitly documented. [Cargo.toml](/C:/dev/sempal/Cargo.toml) defines a Rust 2024 workspace; [README.md](/C:/dev/sempal/README.md) and [docs/ARCHITECTURE.md](/C:/dev/sempal/docs/ARCHITECTURE.md) document the vendored `radiant` runtime/UI layer.
- Repository shape: Explicitly documented. Domain/controller logic lives under `src/`; installer/updater/support tools live under `apps/` and `tools/`; GUI/runtime ownership is split between `src/app_core`, `src/gui*`, and `vendor/radiant`.
- Architectural boundaries: Explicitly documented. [docs/ARCHITECTURE.md](/C:/dev/sempal/docs/ARCHITECTURE.md) and [README.md](/C:/dev/sempal/README.md) keep domain state and UI intent in `src` while `vendor/radiant` owns GUI behavior and runtime internals.
- Test strategy: Strongly implied by code/docs. [docs/TEST.md](/C:/dev/sempal/docs/TEST.md) and the source tree favor deterministic Rust unit/module tests plus targeted integration coverage; GUI desktop automation remains a local/manual lane.
- Canonical local validation commands: Explicitly documented. [docs/README.md](/C:/dev/sempal/docs/README.md), [docs/TEST.md](/C:/dev/sempal/docs/TEST.md), and [AGENTS.md](/C:/dev/sempal/AGENTS.md) define `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1` as the Windows validation ladder.
- Documented priorities: Explicitly documented. [docs/design_principles.md](/C:/dev/sempal/docs/design_principles.md) prioritizes realtime responsiveness, non-blocking execution, reversibility, data integrity, and predictable interaction.
- Explicit non-goals: Explicitly documented. [docs/design_principles.md](/C:/dev/sempal/docs/design_principles.md) says Sempal is a focused creative tool, not a DAW replacement, cloud platform, or attention-retention product.

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for listening to, navigating, editing, and curating local sample libraries with strong emphasis on responsiveness and reversible workflows.
- What the repo appears to be moving toward: Strongly implied by code/docs. Safer file-operation workflows, stronger local/CI guardrails, and a deeper `radiant`-backed native runtime with machine-readable GUI contract coverage.
- What is merely possible but unsupported: broad product-scope expansion, CI-hosted desktop automation promotion, or splitting intentionally centralized domain/runtime contract files only because they are large.

## Ordered Backlog

### 1. [x] Add direct regression coverage for cross-source drag/drop target copy and move rollback paths

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: one user-facing drag/drop path mutates filesystem state, source/target DB rows, and in-memory caches, but it has no direct tests even though the code contains many early exits and rollback branches.
- Evidence:
  - [src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs#L28) runs cross-source move/copy logic in one 160+ line handler with manual rollback on metadata, target registration, and source-row cleanup failures.
  - [src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs#L208) contains copy-specific collision and rollback handling with no local tests.
  - The current drag/drop test coverage found by searching `#[test]` under `src/app/controller/ui/drag_drop_controller/**` covers [source_moves/worker.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/worker.rs) and [folder_moves.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs), but not [drop_targets.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs).
- Recommended change: add focused controller tests for cross-source move success, cross-source copy collision naming, target registration failure rollback, source DB cleanup failure rollback, and missing-source/invalid-target rejection.
- Expected impact: reduces regression risk on a correctness-sensitive path before any structural cleanup or journal-alignment change.
- Risks / tradeoffs: these tests will need fixture scaffolding for multiple sources plus deterministic failure injection around DB registration/removal.
- Dependencies: none
- Suggested validation:
  - targeted drag/drop controller tests (for example `cargo test drag_drop_controller -- --test-threads=1`)
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - user-run confirmation lane: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Date completed: 2026-03-17
- Commit: `8e301fae` (`test(drag-drop): cover cross-source drop targets`)
- Assumptions used:
  - Locking the target source DB with `BEGIN IMMEDIATE` is a safe deterministic way to exercise the target-registration rollback branch because the source-side metadata reads stay unaffected.
  - The source-row cleanup rollback branch belongs with item 2 because the current implementation still needs behavior changes there; item 1 captures the passing rejection, success, collision, and target-write-rollback seams first.
- Validation outcome:
  - `cargo test drag_drop_drop_targets -- --test-threads=1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
  - No plan-order deviation for the completed coverage slice; the remaining source-cleanup-failure branch stays coupled to item 2's implementation work.

### 2. [x] Route cross-source drag/drop target copy and move through the staged file-op journal contract instead of ad-hoc mutations

- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the repository explicitly documents staged journal recovery for copy/move flows, but the cross-source drop-target path still performs direct filesystem and DB mutation without inserting or advancing a recovery journal entry, so a crash mid-operation can leave source/target state without the documented recovery path.
- Evidence:
  - [docs/file_ops_journal_recovery.md](/C:/dev/sempal/docs/file_ops_journal_recovery.md#L8) says copy and move flows can crash after filesystem work and rely on `file_ops_journal` metadata for recovery.
  - [docs/file_ops_journal_recovery.md](/C:/dev/sempal/docs/file_ops_journal_recovery.md#L46) lists journal insertion before filesystem mutation and stage updates after each durable boundary as required invariants.
  - [src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs#L140) performs direct `move_sample_file`, target registration, source-row deletion, and rollback with no `file_ops_journal` use.
  - [src/app/controller/ui/drag_drop_controller/drag_effects/move_transaction.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/move_transaction.rs#L61) and [src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/worker/transaction.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/worker/transaction.rs#L23) already implement the staged journal contract for related move flows.
- Recommended change: reuse or extend the existing staged move/copy helpers so cross-source drag/drop target operations insert journal entries, advance durable stages, and finalize cleanup through the same recovery-aware sequence used elsewhere.
- Expected impact: closes a live crash-integrity hole in a user-facing file operation and brings drag/drop behavior back in line with the repository’s documented recovery contract.
- Risks / tradeoffs: unifying this path with staged helpers will touch controller/file-op boundaries and may require carefully preserving current UI status/reporting behavior.
- Dependencies: item 1
- Suggested validation:
  - the new drop-target regression tests from item 1
  - targeted `source_move` / drag-drop tests with `cargo test source_move -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - user-run confirmation lane: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Date completed: 2026-03-17
- Commit: `9bda0d2e` (`fix(drag-drop): journal cross-source drop targets`)
- Assumptions used:
  - Preserving `locked` during normal success paths is required because the pre-existing drop-target behavior already copied that flag, even though the journal contract only documents tag/loop/playback metadata.
  - Leaving the journal entry in place on post-DB finalize failures is safer than aggressive rollback because reconcile can still finish or clean the operation on the next startup.
- Validation outcome:
  - `cargo test drag_drop_drop_targets -- --test-threads=1` passed
  - `cargo test file_ops_journal -- --test-threads=1` passed
  - `cargo test clipboard_paste -- --test-threads=1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
  - No plan-order deviation; item 2 consumed the remaining source-cleanup rollback branch that item 1 intentionally deferred until the bug fix existed.

### 3. [x] Refresh stale cleanup-hotspot and quality-score artifacts before using them for further prioritization

- Classification: Developer-experience improvement
- Confidence: High
- ROI: Medium-High
- Effort: S
- Why it matters: current planning artifacts still describe a pre-`ci_agent` / pre-refactor tree and now misreport both file-size debt and remaining suppressions, which makes follow-on prioritization less trustworthy.
- Evidence:
  - [tmp/cleanup_audit_hotspots.md](/C:/dev/sempal/tmp/cleanup_audit_hotspots.md) was generated at commit `4869da6f` and still reports five over-budget Rust files plus a `clippy::too_many_arguments` suppression in `compute_worker/execution.rs`.
  - A fresh full scan now passes: `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` returned `[file_budget] OK (836 files checked)` on 2026-03-17.
  - Current line counts place [src/selection/range.rs](/C:/dev/sempal/src/selection/range.rs) at 372 lines and [src/app/controller/playback/tests.rs](/C:/dev/sempal/src/app/controller/playback/tests.rs) at 352 lines, while [tmp/cleanup_audit_hotspots.md](/C:/dev/sempal/tmp/cleanup_audit_hotspots.md) still lists them as 407 and 409.
  - [docs/QUALITY_SCORE.md](/C:/dev/sempal/docs/QUALITY_SCORE.md) still names those same stale file-size hotspots in the “Known gaps” section.
- Recommended change: regenerate [tmp/cleanup_audit_hotspots.md](/C:/dev/sempal/tmp/cleanup_audit_hotspots.md) and align [docs/QUALITY_SCORE.md](/C:/dev/sempal/docs/QUALITY_SCORE.md) with the current branch so future audits stop re-ranking finished work.
- Expected impact: restores trustworthy debt-tracking inputs for later cleanup/perf planning without changing runtime behavior.
- Risks / tradeoffs: purely meta-work; the main risk is accidentally masking a real new hotspot if the refresh is done carelessly.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Date completed: 2026-03-17
- Commit: pending item commit during Phase 2 execution
- Assumptions used:
  - The transient `drop_targets.rs` file-size regression introduced during item 2 needed to be corrected before refreshing the hotspot snapshot; otherwise item 3 would have baked a short-lived artifact into the new baseline.
  - The process-local Git `core.autocrlf=false` override used while rerunning `check_quality_score_drift.ps1` is validation-only and does not change repository policy.
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` passed after splitting the drop-target helper module
  - `cargo test drag_drop_drop_targets -- --test-threads=1` passed after the helper split
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` passed and rewrote `tmp/cleanup_audit_hotspots.md`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` passed when rerun without Git line-ending warning noise
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
  - Tiny prerequisite deviation: split `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs` into a focused child helper module before finalizing the refreshed artifact set, because the new item-2 code temporarily pushed the full-scan budget out of date.

### 4. [ ] Add parity coverage for long-recording waveform aggregation against the standard waveform decode peak/analysis path

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: the live-recording waveform path and the normal decode path currently implement the same peak-bucketing and analysis-subsampling ideas separately, but the recording tests only cover queueing and incremental append/truncation behavior, not parity of the actual math.
- Evidence:
  - [src/app/controller/playback/recording/waveform_loader/aggregation.rs](/C:/dev/sempal/src/app/controller/playback/recording/waveform_loader/aggregation.rs#L100) contains `convert_full_to_peaks`, `consume_data_bytes`, `peak_bucket_size`, `analysis_stride`, and `clamp_sample`.
  - [src/waveform/decode/peaks.rs](/C:/dev/sempal/src/waveform/decode/peaks.rs#L8) and [src/waveform/decode/normalize.rs](/C:/dev/sempal/src/waveform/decode/normalize.rs#L1) define the same helper concepts for the normal waveform decode path.
  - [src/app/controller/playback/recording/waveform_loader/tests.rs](/C:/dev/sempal/src/app/controller/playback/recording/waveform_loader/tests.rs) covers queue replacement, shutdown, decode, truncation, and incremental append, but not bucket-size/analysis-stride parity or long-file peak output equivalence.
- Recommended change: add characterization tests that compare long mono/stereo recording aggregation results against the standard waveform decode peak/analysis behavior, including clamp behavior and rebuild-to-peaks transitions.
- Expected impact: creates a safety net before any deduplication work and reduces the chance that recording preview math silently diverges from normal waveform decode behavior.
- Risks / tradeoffs: some parity assertions may need fixture-building helpers because the two code paths consume different input representations.
- Dependencies: none
- Suggested validation:
  - targeted recording waveform loader tests: `cargo test recording_waveform -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - user-run confirmation lane: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 5. [ ] Extract shared peak/analysis/clamp helpers so recording waveform aggregation and normal waveform decode cannot drift apart

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: the repository now maintains two versions of the same waveform peak/analysis math, which increases the chance of correctness drift and forces any future tuning to be done twice.
- Evidence:
  - [src/app/controller/playback/recording/waveform_loader/aggregation.rs](/C:/dev/sempal/src/app/controller/playback/recording/waveform_loader/aggregation.rs#L331) defines `peak_bucket_size`, [analysis_stride](/C:/dev/sempal/src/app/controller/playback/recording/waveform_loader/aggregation.rs#L335), and [clamp_sample](/C:/dev/sempal/src/app/controller/playback/recording/waveform_loader/aggregation.rs#L345).
  - [src/waveform/decode/peaks.rs](/C:/dev/sempal/src/waveform/decode/peaks.rs#L8) and [src/waveform/decode/normalize.rs](/C:/dev/sempal/src/waveform/decode/normalize.rs#L1) define the same helpers and closely related per-frame peak aggregation loops.
  - [src/app/controller/playback/recording/waveform_loader/decode.rs](/C:/dev/sempal/src/app/controller/playback/recording/waveform_loader/decode.rs#L16) and [incremental_update.rs](/C:/dev/sempal/src/app/controller/playback/recording/waveform_loader/incremental_update.rs#L46) route recording updates through the duplicated helper surface today.
- Recommended change: extract one shared internal peak/analysis helper module with parity tests from item 4, then have both recording aggregation and normal waveform decode call that shared implementation.
- Expected impact: lowers maintenance cost and future correctness drift across two waveform-loading paths without changing public behavior.
- Risks / tradeoffs: the helper boundary needs to stay small and internal; over-abstracting the two call sites would add complexity without much value.
- Dependencies: item 4
- Suggested validation:
  - the parity tests from item 4
  - targeted waveform decode tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - user-run confirmation lane: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 6. [ ] Expand audio-loader stage coverage for stretch, transient generation, and stale-after-stage exits

- Classification: Test gap
- Confidence: High
- ROI: Medium
- Effort: S-M
- Why it matters: the audio-loader stage pipeline drives loaded playback audio and transient-marker generation, but the current tests cover only path validation and early stale-read helpers while later stage behavior remains uncharacterized.
- Evidence:
  - [src/app/controller/playback/audio_loader/stages.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/stages.rs#L189) contains stretch-stage behavior, warning fallback, and post-stretch stale exits.
  - [src/app/controller/playback/audio_loader/stages.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/stages.rs#L304) contains transient generation and stale-after-transient exits.
  - [src/app/controller/playback/audio_loader/tests.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/tests.rs#L17) only exercises path validation, stale request detection, and chunked read behavior.
- Recommended change: add focused tests for successful stretch output, stale-after-stretch short-circuiting, transient-result propagation, and stale-after-transient cancellation.
- Expected impact: strengthens safety around one user-facing audio load path without forcing a refactor first.
- Risks / tradeoffs: some of the stretch/transient paths may require lightweight fixture helpers or deterministic test doubles for renderer output.
- Dependencies: none
- Suggested validation:
  - targeted audio-loader tests: `cargo test audio_loader -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - user-run confirmation lane: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. Should `vendor/radiant/src/app/actions/mod.rs` remain one intentionally centralized compatibility surface?

- Evidence:
  - The module docs in [vendor/radiant/src/app/actions/mod.rs](/C:/dev/sempal/vendor/radiant/src/app/actions/mod.rs#L1) explicitly say `UiAction` is intentionally centralized and should stay broad unless a concrete contract mismatch appears.
  - Current line counts still make it one of the largest live Rust modules, so size-only cleanup heuristics will keep nominating it unless that intent stays explicit.
- Why this matters: future cleanup work could otherwise split a deliberate runtime/host-bridge compatibility seam for the wrong reason.
- Affected files/modules: [vendor/radiant/src/app/actions/mod.rs](/C:/dev/sempal/vendor/radiant/src/app/actions/mod.rs), runtime action routing, host bridge, GUI automation catalog.
- Risk if guessed incorrectly: premature splitting could destabilize the one inspectable action surface shared across runtime, host, and automation code.
- Most conservative provisional assumption: keep `UiAction` centralized and improve organization around it only if a concrete routing or ownership mismatch appears.

### [!] 2. Should `src/selection/range.rs` continue to keep waveform geometry, fades, and gain math together?

- Evidence:
  - The module docs in [src/selection/range.rs](/C:/dev/sempal/src/selection/range.rs#L1) explicitly say the dense file intentionally preserves one waveform-editing domain model.
  - The file is still large enough to attract size-driven cleanup suggestions even though the current docs argue for cohesion.
- Why this matters: splitting a cohesive domain contract just to reduce size would add churn without clear behavioral benefit.
- Affected files/modules: [src/selection/range.rs](/C:/dev/sempal/src/selection/range.rs), waveform selection preview, fade handles, destructive edit flows.
- Risk if guessed incorrectly: over-splitting could scatter one normalized selection/fade contract across several low-value helpers.
- Most conservative provisional assumption: keep the module cohesive unless a clearer subdomain or ownership boundary emerges.

## Rejected Ideas

### [-] 1. Split `vendor/radiant/src/app/actions/mod.rs` immediately

- Why it was considered: it is the largest live Rust file in the current tree by raw line count.
- Why it was rejected: the module now explicitly documents that centralization is intentional because runtime, host bridge, and automation catalog code share one compatibility surface.
- What evidence was missing: any concrete routing, ownership, or regression pain caused by the current centralized enum.

### [-] 2. Split `src/selection/range.rs` immediately

- Why it was considered: it is dense, mathematically non-trivial, and still relatively large.
- Why it was rejected: the file now explicitly documents that geometry, fades, and gain rules are one shared waveform-editing domain model, and the current tests rely on that cohesive contract.
- What evidence was missing: any demonstrated subdomain boundary that would make a split safer or clearer.

### [-] 3. Reopen the old audio-options cleanup lane

- Why it was considered: older audits had identified audio-options control flow as a hotspot.
- Why it was rejected: the current tree already split that area into [controller.rs](/C:/dev/sempal/src/app/controller/playback/audio_options/controller.rs), [normalize.rs](/C:/dev/sempal/src/app/controller/playback/audio_options/normalize.rs), [refresh.rs](/C:/dev/sempal/src/app/controller/playback/audio_options/refresh.rs), [fallback.rs](/C:/dev/sempal/src/app/controller/playback/audio_options/fallback.rs), and dedicated [tests.rs](/C:/dev/sempal/src/app/controller/playback/audio_options/tests.rs).
- What evidence was missing: any live hotspot, failing guardrail, or current-tree regression evidence showing that lane is still one of the highest-value items.
