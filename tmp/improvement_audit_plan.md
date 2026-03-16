# Improvement Audit Plan

Generated: 2026-03-16
Status: Phase 2 in progress. Items are being executed in ranked order.

## Scope

- This document records an evidence-driven Phase 1 audit only.
- No backlog items have been implemented yet in this refresh.
- Recommendations below are ranked in strict execution order by expected ROI.

## Repository Context

- Project purpose: Explicitly documented. `README.md` describes Sempal as a realtime-oriented sample triage and curation app for large local libraries.
- Maturity level: Explicitly documented. `README.md` labels the app early alpha and warns that bugs may modify or delete samples.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace; `vendor/radiant` provides the native GUI/runtime stack described in `README.md` and `docs/ARCHITECTURE.md`.
- Repository shape: Explicitly documented. `README.md` and `docs/ARCHITECTURE.md` split ownership across `src/` domain logic, `src/app_core` action/projection contracts, `vendor/radiant` GUI/runtime/layout work, `tools/` support CLIs, and `docs/`.
- Architectural boundaries: Explicitly documented. `docs/ARCHITECTURE.md` and `README.md` make `app_core` the stable bridge layer and keep renderer/runtime concerns in `vendor/radiant`.
- Test strategy: Explicitly documented. `docs/TEST.md` and `.github/workflows/ci.yml` define diff-aware guardrails, `cargo fmt`, clippy, rustdoc warnings, nextest/doc tests, and Windows PowerShell validation wrappers.
- Canonical local validation commands: Explicitly documented. `docs/TEST.md` and `AGENTS.md` point to `scripts/devcheck.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1` as the normal Windows validation flow.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes realtime behavior, non-blocking execution, data integrity, integrated mouse/keyboard semantics, and undoable edits.
- Explicit non-goals / currently unsupported promotions: Explicitly documented. `docs/gui_test_platform.md` says desktop AIV remains local-only and is not yet stable enough for CI promotion.
- Product direction beyond those documents: Weakly implied / uncertain. The current repository evidence strongly favors correctness, maintainability, and tooling hardening over broad new end-user feature work.

## Ordered Backlog

### 1. [x] Refresh stale cleanup-debt tracking artifacts before using them for further prioritization

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the repo is currently carrying stale audit inputs that misreport already-completed work, which will skew future cleanup or audit planning.
- Evidence:
  - `docs/file_size_budget_allowlist.txt` still allowlists `vendor/radiant/src/gui/native_shell/shots.rs`, but that file no longer exists in the current tree.
  - The same allowlist still includes already-split files such as `src/app/controller/playback/loop_crossfade.rs`, `vendor/radiant/src/gui/native_shell/state/cache_types.rs`, `vendor/radiant/src/gui/native_shell/state/frame_build/chrome/sidebar.rs`, `vendor/radiant/src/gui/native_shell/state/tests/browser_toolbar.rs`, `vendor/radiant/src/gui/native_shell/state/tests/overlays.rs`, and `vendor/radiant/src/gui/native_shell/style/sizing.rs`.
  - `tmp/cleanup_audit_hotspots.md` still reports `src/app/controller/playback/loop_crossfade.rs` as a 502-line hotspot even though the live file is now much smaller.
  - `tmp/cleanup_plan.md` still carries a pending `shots.rs` split item even though that file has already been decomposed.
- Recommended change: refresh the file-size allowlist and hotspot snapshots, and annotate or retire stale parked cleanup entries so future audits start from the live tree instead of obsolete debt data.
- Expected impact: future audits and cleanup plans will stop re-surfacing already-finished work and will rank actual hotspots more accurately.
- Risks / tradeoffs: this is meta-work and does not directly improve runtime behavior, but the effort is small and it protects future prioritization quality.
- Dependencies: none
- Suggested validation: rerun the repo’s cleanup/hotspot generation scripts and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Assumption: the parked cleanup plan should remain historical, but obviously obsolete path references should be annotated so it is not mistaken for a live file map.
  - Validation: reran `scripts/prune_file_size_budget_allowlist.ps1`, reran `scripts/audit_cleanup_hotspots.ps1`, and refreshed the parked cleanup-plan note plus the obsolete `shots.rs` entry.

### 2. [x] Add direct controller coverage for trash auto-move, rollback, cancel, and permanent-delete flows

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: `trash.rs` contains destructive-adjacent behavior with rollback and error aggregation branches, and the file already exposes test-only hooks that are not backed by direct controller coverage.
- Evidence:
  - `src/app/controller/library/trash.rs` is a 396-line controller file with `#[cfg(test)]` behavior such as `run_trash_move_task_with_progress`, `progress_cancel_after`, and a test-fast-path `confirm_warning`.
  - The file owns rollback-sensitive paths around `db.set_missing(...)`, `db.remove_file(...)`, `move_to_trash(...)`, `apply_trash_move_finished(...)`, and recursive `take_out_trash(...)`.
  - A targeted ripgrep over `src/**` test files found no direct tests referencing `move_all_trashed_to_folder`, `move_samples_to_configured_trash`, `take_out_trash`, or `apply_trash_move_finished`.
- Recommended change: add focused controller tests for empty-trash moves, partial move failures with rollback, cancellation after progress, and permanent-delete error aggregation.
- Expected impact: destructive file-move/delete logic becomes safer to refactor and regressions become easier to catch locally.
- Risks / tradeoffs: tests will need temp filesystem/source-db setup and may require lightweight harness helpers.
- Dependencies: none, but this should land before any structural split of `trash.rs`.
- Suggested validation: targeted trash controller tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Assumption: existing trash controller tests already covered the happy-path move, cancel, auto-trash, and successful permanent-delete branches, so the highest-value missing slice was rollback after a failed move.
  - Validation: ran `cargo test trash -- --test-threads=1`.

### 3. [ ] Split `src/app/controller/library/trash.rs` by trash-folder configuration, move orchestration, delete sweep, and result/focus refresh ownership

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the current file mixes UI prompts, config persistence, background move orchestration, direct DB/filesystem mutations, delete sweeps, cache invalidation, and browser refocus behavior in one controller module.
- Evidence:
  - `src/app/controller/library/trash.rs` contains `pick_trash_folder`, `open_trash_folder`, `move_all_trashed_to_folder`, `move_samples_to_configured_trash`, `take_out_trash`, `apply_trash_move_finished`, `apply_trash_folder`, `prepare_trash_folder_for_auto_move`, `ensure_trash_folder_ready`, and `refocus_path_after_trash_move`.
  - The file mixes modal confirmation (`rfd::MessageDialog`), progress UI updates, source DB mutation, filesystem deletion, and cache invalidation in one place.
- Recommended change: extract focused trash submodules or helper modules for folder configuration/readiness, move orchestration, permanent delete sweep, and post-move result handling while keeping the `AppController` surface stable.
- Expected impact: destructive file flows become easier to reason about, review, and test; smaller modules will better match the repo’s ownership guidelines.
- Risks / tradeoffs: splitting too aggressively could obscure flow ownership, so the refactor should preserve one clear controller façade.
- Dependencies: item 2 is the safest precursor so behavior is locked before refactoring.
- Suggested validation: targeted trash tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 4. [ ] Split `vendor/radiant/src/gui/native_shell/layout_adapter/controls.rs` by control-family ownership

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: a hot layout adapter file still mixes unrelated control families and generic slot math, which makes UI geometry regressions harder to localize.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/layout_adapter/controls.rs` is 431 lines.
  - The file defines `BrowserToolbarSections`, `compute_update_action_button_rects`, `compute_sidebar_action_button_rects`, `compute_browser_toolbar_sections`, `compute_rating_filter_chip_rects`, and several shared fixed-slot layout helpers.
  - Local tests already live in a separate `controls_tests` module, which suggests the production logic is ready to split without test discoverability getting worse.
- Recommended change: separate update-panel controls, sidebar action controls, browser-toolbar sectioning, and reusable slot-layout helpers into smaller ownership-based modules.
- Expected impact: UI geometry work becomes more discoverable and less likely to create accidental cross-surface regressions.
- Risks / tradeoffs: nearby helpers share sizing tokens and layout conventions, so the split should preserve a compact shared helper surface instead of duplicating math.
- Dependencies: none
- Suggested validation: relevant `vendor/radiant` native-shell layout tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 5. [ ] Split `vendor/radiant/src/gui/native_shell/layout_adapter/overlays/text.rs` into overlay-family text builders and shared line-layout primitives

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: prompt, progress, and drag overlays are user-visible interruption states, but their text geometry still shares one large mixed-responsibility module.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/layout_adapter/overlays/text.rs` is 438 lines.
  - The file combines `compute_prompt_overlay_text_layout`, `compute_progress_overlay_text_layout`, `compute_drag_overlay_text_layout`, prompt/progress row builders, input-error/counter helpers, and centered-line layout primitives.
  - The local tests cover prompt/progress/drag bounds together, which is useful validation but still leaves the production ownership surface broad.
- Recommended change: keep the existing overlay contract, but split prompt/progress/drag text builders away from shared line-tree and centering helpers.
- Expected impact: overlay regressions become easier to isolate, and future overlay-specific changes stop competing inside one text-geometry hub.
- Risks / tradeoffs: splitting should avoid introducing unnecessary abstraction layers over simple geometry helpers.
- Dependencies: none
- Suggested validation: targeted overlay layout tests in `vendor/radiant` and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 6. [ ] Add direct controller coverage for `handle_analysis_message(...)` progress, clear, enqueue, and cache-invalidation branches

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: the background-analysis message handler concentrates branchy state updates that affect progress UI, cache invalidation, and follow-up job dispatch, but current evidence does not show direct local coverage.
- Evidence:
  - `src/app/controller/library/background_jobs/analysis.rs` is only 186 lines, but almost all of the file’s behavior lives inside one `handle_analysis_message(controller, message)` match.
  - The function handles selected-source scoping, zero-total clear behavior, finished-job cleanup, similarity-prep forwarding, progress snapshot assembly, enqueue follow-up dispatch, and duration-cache invalidation.
  - A targeted ripgrep over `src/**` test files found no direct test coverage for `AnalysisJobMessage`, `handle_analysis_message`, or the specific progress/enqueue branches in this module.
- Recommended change: add controller-level tests for selected-source mismatch, `source_id = None` progress fallback, zero-total clear behavior, finished refresh/invalidation, enqueue-finished redispatch, and `DurationsUpdated` cache eviction.
- Expected impact: analysis-progress behavior becomes safer to change and easier to audit before any further refactor.
- Risks / tradeoffs: tests will need targeted controller setup for runtime/job sender state.
- Dependencies: none
- Suggested validation: targeted analysis background-job tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 7. [ ] Split the inline native-shell contract test hub out of `vendor/radiant/src/gui/native_shell/mod.rs`

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: the root native-shell module is now dominated by a very large inline test block, which makes the real production entry surface harder to scan and maintain.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/mod.rs` is 958 lines.
  - The file contains an inline `mod tests` block with layout contract tests, toolbar hit tests, prompt hit tests, sidebar geometry tests, canonical frame tests, and other unrelated native-shell regression families.
  - The root production module itself is much smaller than the accumulated inline tests, so the file no longer reflects one clear responsibility.
- Recommended change: move the test families into ownership-aligned modules under the existing native-shell test tree, leaving only shared fixtures/helpers at the root when genuinely necessary.
- Expected impact: production entry points become easier to navigate and test failures become easier to map back to one feature family.
- Risks / tradeoffs: moving tests can create temporary churn in fixture/helper imports if not done carefully.
- Dependencies: none
- Suggested validation: `vendor/radiant` native-shell test suite and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 8. [ ] Split `vendor/radiant/src/gui_runtime/native_vello/tests/browser_pointer.rs` by browser interaction family

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: one large browser-pointer regression file now mixes row-selection, toolbar, tab, map, volume, status, and wheel behavior, which reduces test discoverability.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/tests/browser_pointer.rs` is 447 lines.
  - The file covers browser row click modifiers, browser autoscroll, toolbar/rating-filter clicks, browser action buttons, tab routing, map-point focus, top-bar volume interactions, status options clicks, and wheel behavior in one module.
- Recommended change: split the tests into focused modules such as browser rows, browser toolbar/actions, tabs/map interactions, and wheel/scroll behavior.
- Expected impact: future pointer regressions will be easier to triage and new cases can land near the owning behavior instead of in an omnibus file.
- Risks / tradeoffs: over-splitting can make shared setup noisy; keep a small shared fixture helper layer.
- Dependencies: none
- Suggested validation: targeted `browser_pointer`-related vendor tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 9. [ ] Split `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer.rs` by waveform interaction family

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: waveform pointer regressions are packed into one large test file even though the behaviors span distinct editing and transport interaction families.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer.rs` is 477 lines.
  - The file covers click modifiers, edit-selection versus playback-selection behavior, selection-edge resize/smart-scale, clear semantics, shift-handle gestures, narrow-selection edge cases, and anchor precedence in one place.
- Recommended change: split waveform pointer tests into focused modules for selection creation, resize/scale behavior, clear semantics, modifier gestures, and edge-condition handling.
- Expected impact: waveform interaction coverage stays easier to extend without recreating another omnibus regression file.
- Risks / tradeoffs: some setup duplication is likely unless shared waveform fixture builders are extracted carefully.
- Dependencies: none
- Suggested validation: targeted `waveform_pointer` vendor tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

## Open Questions / Missing Definitions

### 1. Should `vendor/radiant/src/app/actions.rs` remain intentionally centralized as the runtime compatibility surface?

- Evidence: the file is still 450 lines, but it is a fully documented `UiAction` enum with serde derives and clear host/runtime contract semantics.
- Why this matters: splitting a deliberate compatibility surface could add churn without improving clarity, especially if external tools or tests depend on one exhaustive enum location.
- Affected files/modules: `vendor/radiant/src/app/actions.rs`, `src/app_core/actions/`, runtime input layers, GUI test tooling.
- Risk if guessed incorrectly: a cleanup refactor could fragment the canonical action contract and make exhaustive matching harder instead of easier.
- Most conservative provisional assumption: treat `UiAction` centralization as intentional until stronger evidence shows ownership pain beyond file length.

### 2. Is `vendor/radiant/src/gui/native_shell/layout.rs` oversized-but-cohesive, or is there a missing ownership split between layout build, hit-testing, and snapshot export?

- Evidence: the file is just over the nominal target, but the responsibilities all sit near the shell layout contract (`ShellLayout`, hit-testing, column lookup, contract snapshot helpers).
- Why this matters: if the file is truly cohesive, splitting it now would be churn; if not, later layout work may keep piling responsibilities into one core contract file.
- Affected files/modules: `vendor/radiant/src/gui/native_shell/layout.rs`, `layout_adapter`, native-shell tests.
- Risk if guessed incorrectly: premature splitting could obscure the central layout contract or, conversely, delay a needed boundary clarification.
- Most conservative provisional assumption: do not queue a split yet; prefer higher-confidence mixed-responsibility files first.

### 3. Is `src/gui_test/packs.rs` meant to stay as a single small pack registry, or is the scenario-pack surface expected to grow into multiple ownership-based modules?

- Evidence: the file is currently moderate in size and only exposes the `contract-smoke` pack plus its scenario builders, while `docs/gui_test_platform.md` still describes the platform as a first slice rather than final coverage.
- Why this matters: future GUI test growth could justify a pack split, but current evidence does not yet show the same ownership pressure as the other hotspots above.
- Affected files/modules: `src/gui_test/packs.rs`, `src/gui_test/aiv/packs.rs`, `docs/gui_test_platform.md`.
- Risk if guessed incorrectly: an unnecessary split would create indirection before the pack taxonomy stabilizes.
- Most conservative provisional assumption: keep the current pack layout until additional packs or clearer ownership boundaries emerge.

## Rejected Ideas

### 1. Split `src/analysis/frequency_domain/stft.rs`

- Why it was considered: the file is still above the preferred size target.
- Why it was rejected: current evidence points to a cohesive DSP stage rather than a mixed-responsibility ownership problem, and no contradictory test/doc signals surfaced in this audit pass.
- What evidence was missing: no strong duplication, boundary confusion, or explicit doc/test mismatch beyond file length.

### 2. Split `src/waveform/model.rs`

- Why it was considered: the file is near the size budget and contains several public waveform model types.
- Why it was rejected: the file still reads as one cohesive waveform data/model surface with local tests and shared sampling semantics.
- What evidence was missing: no clear ownership split or correctness risk beyond size pressure alone.

### 3. Add a new GUI feature backlog slice

- Why it was considered: `docs/gui_test_platform.md` notes that automation coverage is not exhaustive for every micro-control.
- Why it was rejected: the current docs also say the platform is intentionally first-slice and local-only in important areas; the stronger present evidence supports maintainability/test hardening over speculative feature expansion.
- What evidence was missing: no concrete current-repo signal that a new end-user feature or new automation surface should outrank the maintainability and coverage items above.
