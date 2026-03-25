# Improvement Audit Plan

Generated: 2026-03-25
Observed commit: `efd1bbbd`
Status: Phase 2 execution is in progress on 2026-03-25. Items 1-8 are complete, and the ambiguity decisions are locked to the user-approved conservative options for this execution pass.

## Scope

- This document supersedes the previous execution tracker that lived at this path.
- Items are ranked in strict execution order by expected ROI for the repository state observed on 2026-03-25.
- Recommendations stay inside repository-supported direction. Speculative product expansion, broad rewrites, and preference-driven cleanup are excluded.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as a realtime-oriented Rust desktop tool for triaging, auditioning, curating, and editing local audio samples.
- Maturity level: Explicitly documented. `README.md` labels the app early alpha and warns that file operations can modify, rename, or delete user data.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace with the vendored `radiant` GUI/runtime layer plus workspace apps and tools.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` splits domain/controller logic under `src/`, GUI framework/runtime behavior under `vendor/radiant/`, and support binaries under `apps/` and `tools/`.
- Architectural boundaries: Explicitly documented. `README.md` and `docs/ARCHITECTURE.md` keep domain state and UI intent in `src`, while `vendor/radiant` owns widget behavior, layout, hit testing, and rendering coordination.
- Test strategy: Strongly implied by code/docs. `docs/TEST.md` and the source tree emphasize deterministic Rust unit/module tests, targeted controller tests, GUI contract checks, and optional desktop-AIV loops.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, non-blocking execution, predictability, reversibility, and data integrity.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, social network, or attention-retention product.

## Audit Notes

- Knowledge docs currently line up at the repo-entry level: `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1` passed, and `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1` passed.
- High-visibility guardrail status is currently degraded: `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` failed because the full file-size budget is still red while Rust taste invariants are green.
- Guardrail-scope file-size enforcement currently reports nine live violations: `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` flagged `src/app_core/actions/catalog/kinds.rs`, `src/app_core/controller.rs`, `src/app/controller/history.rs`, `src/app/controller/library/selection_export.rs`, `src/app/controller/library/selection_export/background.rs`, `src/app/controller/library/selection_export/selection_export_tests.rs`, `src/app/controller/playback/tests.rs`, `src/app/controller/playback/transport/selection.rs`, and `src/app/controller/tests/drag_drop_drop_targets.rs`.
- The wider hotspot snapshot was refreshed during this audit: `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` rewrote `tmp/cleanup_audit_hotspots.md` at commit `efd1bbbd`, and that broader scan reports 17 over-budget Rust files across the wider audit scope.

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for listening to, navigating, editing, and curating local sample libraries with strong emphasis on responsiveness and reversible workflows.
- What the repo appears to be moving toward: Strongly implied by code/docs. Tighter migration-facing `app_core` seams, stronger GUI/runtime contract coverage, and renewed guardrail discipline around file size, docs, and validation wrappers.
- What is merely possible but unsupported: broad action-model redesigns, mechanical splitting of intentionally cohesive domain modules, or speculative feature work not already implied by the current sample-management and native-runtime direction.

## Ordered Backlog

### 1. [x] Add direct `app_core` controller tests for context-sensitive browser, map, update, and prompt actions

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: `docs/design_principles.md` makes focus-dependent, predictable interaction a core contract, but several mature native-runtime action branches are only covered by representative routing cases or higher-level GUI packs.
- Evidence:
  - `src/app_core/controller.rs:328` routes `CommitFocusedBrowserRow` differently depending on browser focus and falls back to transport playback.
  - `src/app_core/controller.rs:367` and `src/app_core/controller.rs:439` implement multi-branch `ToggleFindSimilarFocusedSample` behavior.
  - `src/app_core/controller.rs:403-405` handles map actions, and `src/app_core/controller.rs:421-433` handles prompt/update/progress actions.
  - `src/app_core/controller/tests/dispatch.rs` only exercises representative routing cases, not these branch-specific outcomes.
  - `docs/gui_migration_parity.md` marks map interactions and update UX as done.
- Recommended change: add focused `app_core` controller tests for browser-focus commit vs transport fallback, map-tab exit on find-similar, no-focused-row status behavior, map focus actions, prompt/update dismiss/cancel flows, and feedback-prompt overlay reset behavior.
- Expected impact: increases confidence in the native runtime's most user-visible controller seam without changing architecture.
- Risks / tradeoffs: tests will need careful fixture setup because many branches depend on focus state, browser selection, and update/prompt state.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test` runs for `src/app_core/controller/tests/*`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `ffa5d5c9`
- Assumptions used: the seeded GUI fixtures do not provide enough similarity-analysis backing data to prove the ANN-backed "create new similar query" success branch at this seam, so the new tests lock down the deterministic browser-focus commit vs transport fallback, map-to-list transition, existing-query clear, no-focus status, map focus staging, progress cancel, feedback prompt reset, and update action branches instead.
- Validation outcome:
  - `cargo test contextual_actions --lib` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Deviation from original plan order: none

### 2. [x] Add deferred selection-export history and crop-completion regression coverage

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the repo treats undo/redo as a first-class interaction primitive, and queued selection exports already participate in deferred history. The current tests exercise file output and progress behaviors, but they do not prove that async success/failure paths create or clear undo state correctly.
- Evidence:
  - `docs/design_principles.md` explicitly requires meaningful actions to be reversible.
  - `src/app/controller/library/selection_export.rs:157-175` registers a pending `SelectionExport` history transaction before queueing the worker job.
  - `src/app/controller/history.rs:357-440` owns the deferred sample-creation transaction lifecycle.
  - `src/app/controller/library/background_jobs/polling/library_handlers.rs:172-216` clears pending history on selection-export failure.
  - `src/app/controller/library/selection_export.rs:330-389` finalizes deferred history on clip-export success, and `src/app/controller/library/selection_export.rs:393-449` applies crop-to-new-sample success.
  - `src/app/controller/library/selection_export/selection_export_tests.rs` has clip/root/slice-batch coverage but no assertions around `undo()`, `redo()`, pending transaction cleanup, or crop completion side effects.
- Recommended change: add focused controller tests for queued export success/failure history behavior and crop-to-new-sample completion behavior, including selection/focus, pending playback, and undo entry creation.
- Expected impact: protects async export flows that modify both filesystem state and meaningful UI state.
- Risks / tradeoffs: these tests will be slightly heavier because they cross worker messaging, history, and controller state restoration.
- Dependencies: none
- Suggested validation:
  - targeted selection-export and history-related `cargo test` runs
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `a1adae7f`
- Assumptions used: the most stable oracles for this item are pending-transaction lifecycle, deferred undo job creation, and crop-completion playback/focus state; the tests therefore simulate failure/crop-completion messages directly where that keeps the history contract deterministic, and they drive the real queued clip-export success path where the repository already exposes a stable background-job harness.
- Validation outcome:
  - `cargo test selection_export_tests --lib` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Deviation from original plan order: none

### 3. [x] Split `src/app_core/controller.rs` along its existing dispatch boundaries

- Classification: Refactor / cleanup
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the live file-size guardrail already flags this file, and the remaining content is naturally partitioned by concern even after waveform-heavy logic moved into `controller/waveform_actions.rs`.
- Evidence:
  - `src/app_core/controller.rs` is 459 lines and is currently flagged by `scripts/check_file_size_budget.ps1 --all`.
  - The file already groups work into separate helpers for transport, browser/sources, map, and prompt/update dispatch (`src/app_core/controller.rs:146`, `:187`, `:398`, and `:413`).
  - `src/app_core/controller/waveform_actions.rs` already proves this module can be split safely by concern.
- Recommended change: keep the public controller facade stable but extract `browser_sources`, `prompt_update`, and startup/frame-preparation helpers into focused sibling modules.
- Expected impact: restores one of the live file-size guardrail failures and makes the controller seam easier to review after item 1 adds branch coverage.
- Risks / tradeoffs: moving dispatch code can create noisy diffs if tests are not in place first.
- Dependencies: item 1
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - targeted `app_core` controller tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `67bc57b2`
- Assumptions used: the safest split keeps transport dispatch, frame preparation, native-controller bootstrapping, and the top-level orchestration in `controller.rs`, while moving the already-isolated browser/source, map, and prompt/update dispatch tables into sibling modules next to the pre-existing waveform action module.
- Validation outcome:
  - `cargo test app_core::controller::tests --lib` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` passed for this item's target by removing `src/app_core/controller.rs` from the live violation list; remaining failures are the later backlog items
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Deviation from original plan order: none

### 4. [x] Split the selection-export cluster and remove the duplicated clip-name generator

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: L
- Why it matters: the live tree has three over-budget files in one workflow cluster, and the clip-name generation logic is currently duplicated between controller and worker paths.
- Evidence:
  - `src/app/controller/library/selection_export.rs` is 479 lines and is flagged by `scripts/check_file_size_budget.ps1 --all`.
  - `src/app/controller/library/selection_export/background.rs` is 473 lines and is flagged by the same guardrail.
  - `src/app/controller/library/selection_export/selection_export_tests.rs` is 435 lines and is also flagged.
  - `src/app/controller/library/selection_export.rs:279-295` defines `next_selection_path_in_dir`.
  - `src/app/controller/library/selection_export/background.rs:488-500` defines a second `next_selection_path_in_dir` with matching suffix logic.
- Recommended change: split the cluster by stable responsibilities (controller queueing/apply, background export operations, and targeted test modules), then extract one shared naming helper used by both direct and queued export paths.
- Expected impact: removes three live file-size violations and lowers drift risk inside a workflow that already spans controller and worker code.
- Risks / tradeoffs: the split should stay narrow and avoid re-litigating the overall selection-export architecture, which is already partially decomposed.
- Dependencies: item 2
- Suggested validation:
  - targeted selection-export tests
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `ae72b6bb`, `0f4a6267`
- Assumptions used: the safest split keeps direct clip-export entry points, queueing, and shared timing instrumentation in `selection_export.rs`, moves UI-thread completion handlers into a sibling `completion.rs` module, moves worker-side naming and entry-registration helpers into `background_recording.rs`, and keeps `AppController::next_selection_path_in_dir` as a thin compatibility shim because the focused clip-export tests already treat it as a stable seam.
- Validation outcome:
  - `cargo test selection_export_tests --lib` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` passed for this item's target by removing `src/app/controller/library/selection_export.rs`, `src/app/controller/library/selection_export/background.rs`, and `src/app/controller/library/selection_export/selection_export_tests.rs` from the live violation list; remaining failures are later backlog items
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Deviation from original plan order: none; the implementation landed as two focused commits so the delegated test-only split stayed disjoint from the primary-agent production refactor.

### 5. [x] Add direct drop-target apply-result tests for cancelled, no-op, and partial-error status paths

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: S-M
- Why it matters: sibling file-operation flows already lock down cancellation and no-op status synthesis, but the drop-target result seam still leaves those user-visible branches implicit.
- Evidence:
  - `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets/apply_result.rs:9-103` contains branches for missing target source, zero-transfer results, cancelled suffixes, and partial errors.
  - `src/app/controller/tests/drag_drop_drop_targets.rs` covers happy-path transfer, collision handling, DB locking, and path rejection, but not these explicit status-text branches.
  - `src/app/controller/tests/drag_drop_sources.rs:129-172` and `src/app/controller/tests/drag_drop_folders.rs:282-355` already cover equivalent source/folder move status branches.
- Recommended change: add direct `apply_drop_target_transfer_result` tests with synthetic result payloads for cancelled, no-op, and partial-error cases.
- Expected impact: brings drop-target transfer status coverage in line with adjacent file-op flows and reduces regression risk in user feedback.
- Risks / tradeoffs: low implementation risk; the main work is building concise synthetic result fixtures.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test drag_drop_drop_targets -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `ee1b49ef`
- Assumptions used: the most stable seam for this item is the direct `apply_drop_target_transfer_result` API, so the tests construct synthetic completed results instead of re-driving the worker pipeline; that keeps the assertions pinned to the user-facing status synthesis branches the audit identified.
- Validation outcome:
  - `cargo test drag_drop_drop_targets --lib` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Deviation from original plan order: none

### 6. [x] Split `src/app/controller/tests/drag_drop_drop_targets.rs` into transfer coverage and drop-target-list coverage

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: S
- Why it matters: this file is a live file-size-budget failure, and it currently mixes cross-source transfer behavior with drop-target panel/list behavior even though the production code is already split by those concerns.
- Evidence:
  - `src/app/controller/tests/drag_drop_drop_targets.rs` is 477 lines and is flagged by `scripts/check_file_size_budget.ps1 --all`.
  - `src/app/controller/tests/drag_drop_drop_targets.rs:34-449` focuses on cross-source copy/move transfer behavior.
  - `src/app/controller/tests/drag_drop_drop_targets.rs:452-523` switches to drop-target list addition/reordering behavior.
  - Production transfer logic lives under `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs` plus `drop_targets/{worker,apply_result}.rs`.
- Recommended change: keep the shared fixture helpers minimal, then move transfer tests and drop-target-list tests into focused sibling modules.
- Expected impact: restores another live guardrail failure with low behavior risk and makes future regression placement easier.
- Risks / tradeoffs: structural churn only; the main risk is over-splitting shared fixtures.
- Dependencies: item 5
- Suggested validation:
  - targeted drag-drop controller tests
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `ac660373`, `2b666fab`, `d383294a`
- Assumptions used: the narrowest useful split is a directory-backed `drag_drop_drop_targets` test module whose root keeps only the tiny `Must` helper while `transfer` owns cross-source transfer/result coverage and `list.rs` owns drop-target panel/reorder behavior; that preserves the existing external module name while removing the oversized flat file.
- Validation outcome:
  - `cargo test drag_drop_drop_targets --lib` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` passed for this item's target by removing the drop-target test file from the live violation list; remaining failures are later backlog items
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Deviation from original plan order: none; the split required a tiny follow-up commit to stage the deleted flat module after the new directory-backed test modules landed, and a later directory-backed follow-up split of `transfer.rs` into `transfer/{mod,workflow,apply_result}.rs` when the file-size guardrail showed that the first replacement module still exceeded the budget.

### 7. [x] Add one post-deadline playback-age persistence regression test

- Classification: Test gap
- Confidence: Medium-High
- ROI: Medium
- Effort: S
- Why it matters: current coverage proves the debounce defers work, but it does not prove the queued playback-age update eventually persists to the database and refreshes UI state after the deadline passes.
- Evidence:
  - `src/app/controller/playback/playback_age.rs:47-107` contains both the deferral logic and the final `db.set_last_played_at(...)` plus `rebuild_browser_lists()` commit path.
  - `src/app/controller/playback/tests.rs:420-439` only checks the pre-deadline no-op behavior.
  - `src/app/controller/tests/browser_actions/focus_navigation.rs:310-329` checks deferral during focus movement, not final persistence.
- Recommended change: add a targeted test that sets the deferred-commit deadline into the past, flushes the pending update, and verifies both queue clearance and persisted `last_played_at` behavior.
- Expected impact: covers a durability-affecting controller seam at low cost.
- Risks / tradeoffs: medium confidence only because the highest-value observable may be DB persistence, UI refresh, or both; the test should choose the smallest stable oracle.
- Dependencies: none
- Suggested validation:
  - targeted playback-age `cargo test` run
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `5f234d1c`
- Assumptions used: the smallest stable oracle for this seam is the queued playback-age commit payload itself plus the persisted `last_played_at` row value, so the new test drives `record_loaded_audio_playback`, forces the deferred deadline into the past, and asserts queue clearance, DB persistence, and the selected-source row state without trying to over-specify broader browser rendering details.
- Validation outcome:
  - `cargo test pending_age_update --lib` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Deviation from original plan order: none

### 8. [x] Split `src/app/controller/playback/tests.rs` by behavior family

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: S-M
- Why it matters: this is another live file-size-budget failure, and the file already overlaps with more focused sibling test modules.
- Evidence:
  - `src/app/controller/playback/tests.rs` is 410 lines and is flagged by `scripts/check_file_size_budget.ps1 --all`.
  - The file mixes playhead helpers, zoom/view math, native selection/view behavior, selection/edit clearing, playback-age debounce, and waveform seek debounce.
  - Adjacent modules already provide narrower homes for related behavior, including `src/app/controller/playback/native_action_tests.rs`, `src/app/controller/playback/waveform_action_tests.rs`, and `src/app/controller/tests/playback_loop/selection_drag.rs`.
- Recommended change: move tests into narrower sibling modules such as `transport`, `view_helpers`, and `playback_age`, instead of continuing to grow the mixed facade bucket.
- Expected impact: restores another live guardrail failure and improves test discoverability.
- Risks / tradeoffs: structural churn only; do not move tests if it obscures the public seam they are actually exercising.
- Dependencies: item 7
- Suggested validation:
  - targeted playback test runs
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `d71561f1`
- Assumptions used: the smallest stable split is a directory-backed `playback::tests` module where lightweight label/playhead helpers live in `view_helpers.rs`, deferred playback-age coverage lives in `playback_age_tests.rs`, deferred seek coverage lives in `seek_tests.rs`, and the remaining zoom/native waveform behavior stays in `waveform_actions.rs` with its local `seed_waveform_for_zoom` helper.
- Validation outcome:
  - `cargo test playback::tests --lib` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` passed for this item's target by removing `src/app/controller/playback/tests.rs` from the live violation list; remaining failures are later backlog items
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Deviation from original plan order: a tiny prerequisite follow-up split was needed in `src/app/controller/tests/drag_drop_drop_targets/transfer.rs` after the file-budget check revealed the earlier item-6 replacement module still exceeded the limit.

### 9. [ ] Add catalog-to-runtime completeness checks for action semantics before considering wider consolidation

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: the host action catalog is documented as canonical, but action semantics are still maintained in multiple manual tables across catalog, controller dispatch, bridge classification, bridge invalidation, and history support.
- Evidence:
  - `docs/gui_test_platform.md` calls the host action catalog canonical.
  - `src/app_core/actions/catalog/entries.rs:60-95` defines catalog metadata and coverage policy.
  - `src/app_core/controller.rs:146`, `:187`, `:225`, `:398`, and `:413` maintain manual dispatch groups.
  - `src/app_core/native_bridge/action_classification.rs` and `src/app_core/native_bridge/invalidation.rs` maintain additional action semantics.
  - `src/app/controller/history.rs:459-500` maintains a separate history-support compatibility table.
- Recommended change: add explicit guard tests that verify every cataloged action has the expected runtime semantic companions (classification, invalidation handling, and history support when policy requires it), while keeping the current architecture intact.
- Expected impact: reduces semantic drift risk without committing the repo to a broader action-model redesign.
- Risks / tradeoffs: new tests will surface existing inconsistencies quickly; that may create follow-up cleanup work before any bigger architectural change is justified.
- Dependencies: item 1
- Suggested validation:
  - `src/app_core/actions/tests.rs`
  - native-bridge semantic/unit tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 10. [ ] Isolate the history-policy compatibility matcher from `history.rs` without disturbing the snapshot/transaction core

- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Medium-Low
- Effort: S
- Why it matters: `history.rs` is over budget, but the main snapshot/restore and deferred-transaction logic looks intentionally cohesive. The lowest-risk split is the catalog-compatibility tail that duplicates action-history knowledge already present elsewhere.
- Evidence:
  - `src/app/controller/history.rs` is 472 lines and is flagged by `scripts/check_file_size_budget.ps1 --all`.
  - `src/app/controller/history.rs:459-500` hardcodes `GuiActionKind` support for `GuiHistoryPolicy`.
  - `src/app_core/actions/catalog/entries.rs:60-91` already assigns history policy for those action kinds.
  - `src/app_core/actions/tests.rs:93-104` exists to keep the two tables aligned.
  - The rest of `src/app/controller/history.rs` is a cohesive snapshot/deferred-history owner with existing controller tests under `src/app/controller/tests/history_transactions.rs`.
- Recommended change: extract only the policy-compatibility matcher into a smaller dedicated module or helper near the catalog/controller boundary, and keep the core snapshot/deferred-history implementation together.
- Expected impact: restores another live guardrail failure with minimal ownership churn.
- Risks / tradeoffs: medium confidence because moving the matcher without improving surrounding tests can still create blind spots; item 9 should land first or together.
- Dependencies: item 9
- Suggested validation:
  - `src/app_core/actions/tests.rs`
  - `src/app/controller/tests/history_transactions.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. Is the host action catalog intended to become the single semantic source for runtime action behavior, or only the canonical metadata registry?

- Evidence:
  - `docs/gui_test_platform.md` calls the host action catalog canonical.
  - Action semantics are still duplicated across `src/app_core/actions/catalog/entries.rs`, `src/app_core/controller.rs`, `src/app_core/native_bridge/action_classification.rs`, `src/app_core/native_bridge/invalidation.rs`, and `src/app/controller/history.rs`.
- Why this matters: item 9 can safely add stronger completeness tests now, but any future consolidation work depends on whether maintainers want the catalog to drive semantics or merely describe them.
- Affected files/modules: `src/app_core/actions/catalog/*`, `src/app_core/controller.rs`, `src/app_core/native_bridge/*`, `src/app/controller/history.rs`
- Risk if guessed incorrectly: either unnecessary architectural churn or continued semantic drift between manual tables.
- Most conservative provisional assumption: treat the catalog as the canonical metadata surface and add cross-table guard tests first, without attempting a broader redesign.
- Resolution for this execution (2026-03-25): treat the catalog as canonical metadata and add stronger completeness guards before any broader consolidation.

### [!] 2. Should intentionally cohesive, canonical modules that exceed the 400-line budget receive narrow documented exceptions or continue to be treated as split candidates?

- Evidence:
  - `src/app_core/actions/catalog/kinds.rs` is over budget in the current full-scan guardrail.
  - `src/selection/range.rs` explicitly documents why its waveform geometry, fade, and gain rules stay together.
  - `src/app/controller/playback/transport/selection.rs` keeps one drag/snap/retarget domain plus local tests and separate loop tests.
- Why this matters: the current guardrail pressure will keep resurfacing these files unless the repo clarifies whether cohesion can trump the budget in narrow cases.
- Affected files/modules: `src/app_core/actions/catalog/kinds.rs`, `src/selection/range.rs`, `src/app/controller/playback/transport/selection.rs`, `docs/file_size_budget_allowlist.txt`
- Risk if guessed incorrectly: low-value mechanical splits, or permanent recurring debt that never becomes explicitly accepted.
- Most conservative provisional assumption: reject mechanical splits for now, and only use a documented exception if maintainers explicitly decide those files should remain centralized.
- Resolution for this execution (2026-03-25): treat the 400-line budget as a strong default with rare documented exceptions for clearly cohesive modules.

### [!] 3. Are higher-level GUI scenario/AIV packs intended to be sufficient coverage for mature map/update/prompt behaviors, or should those flows also have direct controller-seam tests?

- Evidence:
  - `docs/gui_migration_parity.md` marks map interactions and update UX as done.
  - `src/gui_test/packs/map.rs` and `src/gui_test/aiv/packs/cases/update.rs` cover higher-level paths.
  - `src/app_core/controller/tests/dispatch.rs` only has representative routing cases for the same action families.
- Why this matters: the answer changes whether app-core controller branch coverage is considered missing correctness protection or just redundant layering.
- Affected files/modules: `src/app_core/controller.rs`, `src/app_core/controller/tests/*`, `src/gui_test/packs/*`, `src/gui_test/aiv/packs/cases/*`
- Risk if guessed incorrectly: either under-testing a key controller seam or investing in redundant low-value tests.
- Most conservative provisional assumption: add a few direct controller-seam tests for the branch-heavy, focus-sensitive cases and continue relying on GUI packs for broader end-to-end behavior.
- Resolution for this execution (2026-03-25): keep GUI/AIV packs for end-to-end coverage and add direct controller-seam tests for branch-heavy cases.

## Rejected Ideas

### [-] 1. Redesign the `app_core` / `radiant` action model

- Why it was considered: action semantics are duplicated across catalog, controller, bridge, and history tables.
- Why it was rejected: the repo is in a migration-stabilization posture, and the architecture docs favor thin adapters and controlled alias surfaces over a new cross-cutting abstraction.
- What evidence was missing: any repository-local sign that maintainers want a broader architectural reset rather than tighter guardrails.

### [-] 2. Split `src/app_core/actions/catalog/kinds.rs` immediately

- Why it was considered: it is a live file-size-budget failure.
- Why it was rejected: it is the stable payload-free action identity surface, the docs call this catalog area canonical, and there is no concrete correctness or ownership defect tied to the current file shape.
- What evidence was missing: a real bug, maintenance failure, or repeated change-friction signal specifically caused by keeping `GuiActionKind` centralized.

### [-] 3. Split `src/selection/range.rs` immediately

- Why it was considered: it remains over the broader cleanup snapshot budget.
- Why it was rejected: the file explicitly documents why range geometry, fades, and gain evaluation stay together, and the audit found no contradictory ownership or correctness evidence.
- What evidence was missing: a clearer subdomain boundary, a repeated maintenance problem, or a correctness defect tied to the current cohesion.

### [-] 4. Split `src/app/controller/playback/transport/selection.rs` mechanically

- Why it was considered: it is a live file-size-budget failure in the enforced scope.
- Why it was rejected: the module currently reads as one cohesive selection-drag and playback-retarget domain, already includes local unit tests, and has adjacent loop tests in a separate module.
- What evidence was missing: a concrete ownership mismatch or a clearly separate subdomain worth extracting now.
