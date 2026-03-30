# Improvement Audit Plan

Generated: 2026-03-30
Observed superproject commit: `4be4051e`
Observed `vendor/radiant` commit: `dd22ac1c`
Observed workspace state: clean worktree in the superproject at audit time.
Status: Phase 2 execution is paused after the safe executable backlog was burned down and revalidated on 2026-03-30. Items 1 and 3 are complete; items 2, 5, and 6 remain clarification-gated; item 4 stays blocked on item 2; item 7 stays blocked on item 6; and item 8 is now blocked by clarification plus unrelated dirty files (`src/app_core/controller/tests/browser_sources.rs` and `vendor/radiant/**`) after the clean root-side scope was split and validated.

## Scope

- This refresh supersedes the previous 2026-03-29 execution record that lived at this path.
- Findings are ranked in strict execution order by expected ROI for the current live tree, not by category.
- Recommendations stay inside repository-supported direction. Broad rewrites, speculative features, and preference-only cleanup are excluded.

## Decision Log

- 2026-03-30: Rebuilt the backlog from the current clean `next` tree after the documented preflight surfaced a new migration-boundary failure at `HEAD`.
- 2026-03-30: Kept already-fixed 2026-03-29 items out of the new backlog unless the live tree still showed an active gap.
- 2026-03-30: Started Phase 2 after user confirmation and landed item 1 in commit `7fa92ac6`.
- 2026-03-30: Landed item 3 after routing crop-to-new-sample through the shared pending-history snapshot path and adding focused crop-history regression coverage.
- 2026-03-30: While validating item 3, the repo-wide serial lib suite exposed a pre-existing `gui_test` test-harness instability tied to repeated `default` fixture usage; tightening those unit tests and the shell smoke pack to deterministic named fixtures restored stable `gui_test::` and `ci_agent` coverage without changing the product contract under audit.
- 2026-03-30: Burned down the clean root-side file-size debt in phased splits across `analysis::audio`, controller runtime/audio-loading helpers, and multiple oversized test modules, landing commits `30326f28`, `ee6cd2b7`, and `38bed509` plus the current follow-up root-side split pass.
- 2026-03-30: Re-ran the full serial lib suite and `devcheck`; remaining file-budget debt is now limited to the clarification-gated `src/gui_test/runner.rs`, the user-dirty `src/app_core/controller/tests/browser_sources.rs`, and user-dirty `vendor/radiant/**` hotspots.
- 2026-03-30: Re-ran `scripts/ci_agent.ps1` after the follow-up root-side split pass; the agent-safe lane is green, and the remaining backlog is now purely clarification-gated or blocked on unrelated dirty files.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as an early-alpha Rust desktop tool for triaging, auditioning, editing, and curating local audio samples.
- Maturity level: Explicitly documented. `README.md` warns that the app is early alpha and can modify, rename, or delete sample-library files.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace with the root `sempal` crate, workspace apps/tools, and the vendored `radiant` GUI/runtime submodule.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` splits domain/controller logic under `src/`, migration-facing projection/runtime glue under `src/app_core` and `src/gui_runtime`, GUI/runtime behavior under `vendor/radiant/`, and support apps/tools under `apps/` and `tools/`.
- Architectural boundaries: Explicitly documented. `docs/ARCHITECTURE.md` says domain state and UI intent belong in `src`, while `vendor/radiant` owns widget behavior, layout, hit testing, input routing, and rendering coordination.
- Test strategy: Strongly implied by code/docs. `docs/TEST.md` and `.github/workflows/ci.yml` center the repo on deterministic Rust unit/module tests, `cargo nextest`, targeted GUI contract tests, and optional desktop-AIV loops.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, non-blocking execution, predictability, reversibility, and data integrity.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, social network, or attention-retention product.

## Audit Notes

- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` failed during the mandatory preflight because `scripts/check_migration_boundary.ps1` found live `crate::app::` references under `src/app_core/**` outside `src/app_core/app_api.rs`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` reproduced the same failure directly.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` originally failed with 29 active-scope file-budget violations across the root repo and `vendor/radiant/src`; after the safe root-side burn-down it now fails with 16 remaining violations limited to one dirty root test, the clarification-gated `src/gui_test/runner.rs`, and dirty `vendor/radiant` files.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md`, which now reports 59 broader over-budget Rust files, 1263 scanned Rust files, and several non-allowlisted production hotspots.
- `scripts/check_docs_index.ps1` and `scripts/check_codeowners_coverage.ps1` still pass, so the current top issues are code/contract debt rather than missing index wiring.
- Item 3 validation now passes:
  - `cargo test crop_export_history_tests --lib`
  - `cargo test apply_selection_crop_export_success_restores_focus_playback_and_undo_state --lib`
  - `cargo test crop_to_new_sample_queues_export_and_async_loads_new_clip --lib`
  - `cargo test gui_test:: --lib -- --test-threads=1`
  - `cargo test -p sempal --lib -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Item 8 safe-scope validation now passes:
  - `cargo test -p sempal --lib -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` (still red, but only for the dirty/blocked files listed above)

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for listening to, navigating, editing, and curating local sample libraries with strong emphasis on responsiveness and reversible workflows.
- What the repo appears to be moving toward: Strongly implied by code/docs. Tighter `app_core` migration boundaries, broader snapshot-based undo coverage for meaningful UI workflows, a truthful semantic GUI test platform, and ongoing file-size/hotspot burn-down with root-side clean debt kept small.
- What is merely possible but unsupported: broad `app_core` redesigns, replacing the vendored runtime strategy, or promoting unstable desktop-AIV coverage into default CI now.

## Ordered Backlog

### 1. [x] Restore the `app_core` migration boundary in native-shell projection helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the documented preflight/CI path is red at `HEAD`, and the live tree currently violates the repo’s explicit rule that direct `crate::app::` crossings stay isolated to `src/app_core/app_api.rs`.
- Evidence:
  - `scripts/check_migration_boundary.ps1` and `scripts/check_migration_boundary.sh` only allow direct `crate::app::` references in `src/app_core/app_api.rs`.
  - `src/app_core/native_shell.rs:254` and `src/app_core/native_shell.rs:342` still reference `crate::app::state::WaveformSliceBatchProfile` and `crate::app::state::UiPoint`.
  - `src/app_core/native_shell/browser_projection/row_window.rs:169` still references `crate::app::state::BrowserDuplicateCleanupState`.
  - `src/app_core/native_shell/waveform_projection.rs:187` and `src/app_core/native_shell/waveform_projection.rs:230` still reference `crate::app::state::WaveformSliceBatchProfile`.
  - `docs/gui_migration_parity.md:82`, `docs/gui_migration_parity.md:86`, and `docs/gui_migration_parity.md:161` claim legacy crossings are centralized and no blockers remain, which no longer matches the live tree.
- Recommended change: route these projection helpers through `app_core::state` or `app_core::app_api::state` aliases, then refresh any migration doc lines that overstate the current boundary status.
- Expected impact: restores the mandatory preflight gate, re-aligns the live tree with the documented ownership boundary, and reduces future migration drift in `app_core`.
- Risks / tradeoffs: low; the main risk is fixing the import-path violation without clarifying the longer-term type-ownership direction, which could allow similar drift to reappear later.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-30
- Commit: `7fa92ac6` (`fix: restore app_core migration boundary`)
- Assumptions: routing these projection helpers back through `app_core::app_api::state` is the intended minimal repair for the documented boundary, even though broader type-ownership narrowing may remain future work.
- Validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Plan order deviation: none

### 2. [!] Clarify whether compare-anchor state is part of the “meaningful UI” undo/redo contract

- Classification: Product-definition gap
- Confidence: High
- ROI: High
- Effort: S-M
- Why it matters: the design principles promise uniform undo/redo coverage for meaningful in-session workflows, but compare-anchor state currently mutates outside `MeaningfulUiSnapshot` and its intended reversibility is not documented.
- Evidence:
  - `docs/design_principles.md:126-134` says meaningful in-session actions should be uniformly reversible via undo/redo.
  - `src/app/controller/playback/compare_anchor.rs:7`, `src/app/controller/playback/compare_anchor.rs:38`, `src/app/controller/playback/compare_anchor.rs:120`, `src/app/controller/playback/compare_anchor.rs:146`, and `src/app/controller/playback/compare_anchor.rs:168` set, replay, clear, rewrite, and assign compare-anchor state.
  - `src/app/controller/history.rs:46-80` and `src/app/controller/history.rs:133-162` define and populate `MeaningfulUiSnapshot`, but do not capture compare-anchor fields.
  - `src/app/controller/tests/compare_anchor.rs` covers set/replay/missing-anchor behavior, but there is no undo/redo coverage for compare-anchor state.
- Recommended change: decide whether compare-anchor is meaningful undo state. If yes, add it to snapshot capture/restore and cover it with history tests. If not, document the explicit exemption in the behavior/design docs.
- Expected impact: resolves a live ambiguity in a user-facing audition workflow and prevents future undo/redo changes from silently widening or narrowing the contract.
- Risks / tradeoffs: medium; treating compare-anchor as undoable broadens snapshot churn, while exempting it weakens the repo’s “uniform undo” story.
- Dependencies: none
- Suggested validation:
  - focused compare-anchor/history undo tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: Yes

### 3. [x] Put crop-to-new-sample undo/redo on the same snapshot-restore path as other async selection exports

- Classification: Bug fix
- Confidence: Medium
- ROI: High
- Effort: S-M
- Why it matters: one export lane already uses `attach_meaningful_ui_restore` for async history completion, but crop-to-new-sample currently appears to revert only the file operation while leaving selection/focus/playback restoration to incidental side effects instead of the explicit snapshot contract.
- Evidence:
  - `src/app/controller/library/selection_export.rs:165-173` begins a pending sample-creation history transaction for the browser clip export path.
  - `src/app/controller/history.rs:377-401` finalizes pending sample creation with `attach_meaningful_ui_restore(...)`.
  - `src/app/controller/library/selection_export/completion.rs:61-109` handles crop-to-new-sample completion by mutating browser selection, waveform focus, playback resume, and status, then pushing `crop_new_sample_undo_entry(...)` directly.
  - `src/app/controller/library/selection_edits/undo_entries.rs:55-106` builds `crop_new_sample_undo_entry` with deferred file jobs only; there is no post-undo/post-redo meaningful-UI restore hook.
  - `src/app/controller/ui/file_ops.rs:232-243` only runs `run_post_undo` / `run_post_redo` hooks when the undo entry provides them.
- Recommended change: route crop-to-new-sample through the same pending-history snapshot attach path as the browser clip export lane, and add a focused regression test that asserts undo/redo restores the expected selection/focus/playback context.
- Expected impact: makes one reversible editing workflow consistent with the repo’s broader snapshot-based history model and reduces the chance of UI-context drift after crop undo/redo.
- Risks / tradeoffs: medium; if the crop lane intentionally wants different post-undo focus behavior, that intention needs to be captured explicitly before the history path is unified.
- Dependencies: item 2 if compare-anchor is deemed part of the meaningful snapshot contract
- Suggested validation:
  - targeted crop-export/history undo tests in one cargo process
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-30
- Commit: `95364b39` (`fix: unify crop export history restore`)
- Assumptions:
  - crop-to-new-sample should follow the same pending sample-creation snapshot contract as other async selection exports.
  - tightening `gui_test` unit coverage to deterministic named fixtures is an acceptable validation prerequisite because those assertions do not rely on persisted-startup behavior.
- Validation:
  - `cargo test crop_export_history_tests --lib`
  - `cargo test apply_selection_crop_export_success_restores_focus_playback_and_undo_state --lib`
  - `cargo test crop_to_new_sample_queues_export_and_async_loads_new_clip --lib`
  - `cargo test gui_test:: --lib -- --test-threads=1`
  - `cargo test -p sempal --lib -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Plan order deviation: fixed a small prerequisite `gui_test` validation instability before landing item 3 so the documented serial lib suite and `ci_agent` lane could run green again.

### 4. [~] Deepen regression coverage for `MeaningfulUiSnapshot` restore and async history completion

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: `history.rs` restores a wide surface of source selection, folder state, browser focus/selection/autoscroll, loaded sample state, waveform ranges/view/cursor/loop state, and async completion hooks, but the current direct coverage still only exercises a narrow subset.
- Evidence:
  - `src/app/controller/history.rs:167-252` restores selected source, folder state, browser selection/focus/autoscroll, selected/loaded wav, waveform selection/edit selection/view/cursor, and loop state.
  - `src/app/controller/history.rs:350-419` attaches meaningful-UI restore hooks around async overwrite and sample-creation completion.
  - `src/app/controller/tests/history_transactions.rs:14-118` only covers four basic undo/redo cases.
  - `src/app/controller/library/selection_export/selection_export_tests/waveform_selection_export_tests.rs:157-250` and `src/app/controller/library/selection_export/selection_export_tests/waveform_selection_export_tests.rs:364-385` cover pending transaction registration/cancellation, but not the richer post-undo/post-redo UI restore surface.
- Recommended change: add focused history tests for snapshot capture/restore of the full meaningful surface and async completion hooks, using small table-driven cases instead of one giant scenario file.
- Expected impact: hardens one of the repo’s core reversibility contracts without changing product direction.
- Risks / tradeoffs: medium; the tests need disciplined fixtures so they validate behavior-level outcomes instead of internal representation details.
- Dependencies: items 2 and 3
- Suggested validation:
  - targeted history and selection-export tests in one cargo process
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 5. [!] Define the retention and pruning policy for unmatched `pending_wav_renames` rows

- Classification: Product-definition gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: pending rename rows preserve tags and metadata for quick/deep rename reconciliation, but the current repo still does not define how long unmatched rows should survive or when they should be pruned.
- Evidence:
  - `src/sample_sources/scanner/scan_diff.rs:125-126` stages every leftover missing row as a pending rename during quick scans.
  - `src/sample_sources/db/pending_renames.rs:132-228` only clears pending rows when they are claimed uniquely or when a live-path upsert conflicts with them.
  - `src/sample_sources/scanner/scan_hash.rs:21-120` only clears retained rows when deep-hash reconciliation finds a unique match.
  - `src/sample_sources/scanner/scan/tests.rs:258-290` intentionally leaves ambiguous large-file renames in `pending_wav_renames`.
  - Search across `src/sample_sources/**` did not find a broader TTL, hard-rescan prune, or stale-row cleanup path.
- Recommended change: document one explicit retention/pruning policy for pending renames, then enforce it in the scanner/DB helpers and add tests for hard-rescan, ambiguous-rename, and eventual-prune behavior.
- Expected impact: removes a silent trust-model ambiguity around whether metadata for deleted/moved samples is preserved temporarily or indefinitely.
- Risks / tradeoffs: medium; an aggressive prune policy can lose intended metadata preservation, while indefinite retention can accumulate stale rows and surprising future matches.
- Dependencies: none
- Suggested validation:
  - targeted scanner/db tests for quick scan, deep scan, ambiguous rename, and prune behavior
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: Yes

### 6. [!] Make `GuiScenarioStep::CaptureSnapshot` truthful or remove it from the supported scenario contract

- Classification: Bug fix
- Confidence: High
- ROI: Medium-High
- Effort: S-M
- Why it matters: the public GUI scenario schema exposes a labeled capture step, but the runner currently treats that step as a silent no-op even though the docs and CLI present `run-scenario` as a supported workflow surface.
- Evidence:
  - `src/gui_test/scenario.rs:22-25` defines `GuiScenarioStep::CaptureSnapshot { label }` and documents it as capturing the latest automation snapshot.
  - `src/gui_test/runner.rs:63-66` handles `GuiScenarioStep::CaptureSnapshot { .. }` with an empty match arm.
  - `src/gui_test/artifacts.rs:80` only stores one final `automation_snapshot`, so there is nowhere for intermediate labeled captures to land today.
  - `docs/gui_test_platform.md:122-135` and `tools/gui-test-cli/src/main.rs:35-57` present `run-scenario` and `run-scenario-pack` as normal supported entrypoints.
- Recommended change: either implement labeled intermediate snapshot capture in the artifact/report path or remove/deprecate the step so unsupported behavior is not silently advertised.
- Expected impact: makes the GUI scenario contract honest and prevents downstream tooling from depending on a no-op feature.
- Risks / tradeoffs: medium; adding intermediate captures expands the artifact schema, while removing the step may require a migration path for unpublished consumers.
- Dependencies: none
- Suggested validation:
  - targeted `src/gui_test` runner tests for the chosen behavior
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: Yes

### 7. [~] Split `src/gui_test/runner.rs` after the capture-step contract is settled

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: the runner is currently over budget and mixes fixture bootstrap, scenario execution, assertion polling, artifact assembly, step labeling, and local tests in one file.
- Evidence:
  - `src/gui_test/runner.rs` is currently 448 lines and fails the full file-size budget check.
  - `src/gui_test/runner.rs:54-99` runs scenarios and timing capture.
  - `src/gui_test/runner.rs:101-143` assembles artifact bundles and current snapshots.
  - `src/gui_test/runner.rs:145-219` contains assertion evaluation.
  - `src/gui_test/runner.rs:237-243` handles step labeling, including the currently misleading `CaptureSnapshot` label.
  - `src/gui_test/runner.rs:246-444` embeds a sizeable local test module in the same file.
- Recommended change: split the file around `execution`, `assertions`, and `artifact/bundle` responsibilities after item 6 clarifies whether capture steps remain part of the contract.
- Expected impact: restores the repo’s file-size and single-responsibility discipline in an actively evolving GUI-test-platform module.
- Risks / tradeoffs: medium; moving tests and helpers can create temporary churn if the split is not anchored to stable boundaries.
- Dependencies: item 6
- Suggested validation:
  - targeted `cargo test gui_test:: -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
- Product clarification required: No

### 8. [~] Burn down the unsupported live file-size budget debt in current production modules

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: L
- Why it matters: the full-scan budget is red again, and the current unsupported debt is no longer confined to a few intentional allowlist entries. Several active production modules now sit above the 400-line budget, which is already reflected in the quality score and hotspot snapshot.
- Evidence:
  - `scripts/check_file_size_budget.ps1 -All` originally reported 29 active-scope violations.
  - After the current root-side burn-down, `scripts/check_file_size_budget.ps1 -All` now reports only 16 remaining violations:
    - `src/app_core/controller/tests/browser_sources.rs` (dirty in the current worktree)
    - `src/gui_test/runner.rs` (still blocked behind item 6 and item 7)
    - `vendor/radiant/src/gui/native_shell/layout_adapter/sidebar_header.rs`
    - `vendor/radiant/src/gui/native_shell/layout_adapter/waveform_annotations.rs`
    - `vendor/radiant/src/gui/native_shell/state/frame_build/overlay/focus.rs`
    - `vendor/radiant/src/gui/native_shell/state/hit_testing/waveform.rs`
    - `vendor/radiant/src/gui/native_shell/state/tests/browser_scrollbars.rs`
    - `vendor/radiant/src/gui/native_shell/state/tests/chrome_layout/waveform_toolbar.rs`
    - `vendor/radiant/src/gui/native_shell/state/tests/frame_build.rs`
    - `vendor/radiant/src/gui/native_shell/state/tests/overlay_controls.rs`
    - `vendor/radiant/src/gui/native_shell/state/tests/sidebar.rs`
    - `vendor/radiant/src/gui/native_shell/state/tests/waveform_selection.rs`
    - `vendor/radiant/src/gui_runtime/native_vello/runtime_events/pointer.rs`
    - `vendor/radiant/src/gui_runtime/native_vello/tests/key_bindings.rs`
    - `vendor/radiant/src/gui_runtime/native_vello/tests/runtime_core.rs`
    - `vendor/radiant/src/gui_runtime/native_vello/text_bpm.rs`
  - The root-side clean offenders were split into focused modules across `src/analysis/audio/exact_duplicates/**`, `src/app/controller/state/runtime/**`, `src/app/controller/library/wavs/audio_loading.rs`, `src/app/controller/library/wavs/entry_mutation/**`, `src/app/controller/library/background_jobs/polling/tests/**`, `src/app/controller/library/slices/slices_tests/**`, `src/app/controller/library/selection_export/selection_export_tests/waveform_selection_export_tests/**`, `src/app/controller/playback/tests/waveform_actions/**`, `src/app/controller/tests/drag_drop_folders/**`, `src/app/controller/tests/browser_actions/row_actions/**`, `src/app/controller/tests/folders_core/rename_delete_recovery/**`, `src/app_core/controller/tests/waveform/**`, and `src/app_core/native_bridge/tests/bridge_runtime/**`.
  - `docs/file_size_budget_allowlist.txt` still keeps cohesive exceptions explicit, so the remaining debt is now limited to blocked or unrelated-dirty files rather than the broader root tree.
- Recommended change: keep item 8 blocked until the user either resolves the `GuiScenarioStep::CaptureSnapshot` contract for `src/gui_test/runner.rs` or allows work in the currently dirty `src/app_core/controller/tests/browser_sources.rs` and `vendor/radiant/**` files.
- Expected impact: the clean root repo is now back under the budget guardrail except for blocked/dirty files, which sharply reduces future review noise and local structural debt.
- Risks / tradeoffs: low for the completed root-side splits; the remaining risk is mostly coordination risk if blocked or user-dirty files are edited without a clarified owner or contract.
- Dependencies: item 7 for `src/gui_test/runner.rs`; a clean or explicitly approved worktree for `src/app_core/controller/tests/browser_sources.rs` and `vendor/radiant/**`
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted module/unit tests in one cargo process per split
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Blocked: 2026-03-30
- Commits:
  - `30326f28` (`refactor: split audio loading module`)
  - `ee6cd2b7` (`refactor: split slice tests into focused modules`)
  - `38bed509` (`refactor: split polling tests into focused modules`)
- Assumptions:
  - behavior-preserving file splits inside the clean root-side test and helper modules are still in scope for item 8 even though the original title emphasized production modules.
  - the unrelated dirty files in `src/app_core/controller/tests/browser_sources.rs` and `vendor/radiant/**` should not be touched without explicit coordination.
- Validation:
  - `cargo test exact_duplicate --lib -- --test-threads=1`
  - `cargo test app::controller::state::runtime::performance::tests:: --lib -- --test-threads=1`
  - `cargo test entry_mutation --lib -- --test-threads=1`
  - `cargo test waveform_selection_export_tests --lib -- --test-threads=1`
  - `cargo test waveform_actions:: --lib -- --test-threads=1`
  - `cargo test drag_drop_folders:: --lib -- --test-threads=1`
  - `cargo test row_actions:: --lib -- --test-threads=1`
  - `cargo test rename_delete_recovery:: --lib -- --test-threads=1`
  - `cargo test app_core::controller::tests::waveform:: --lib -- --test-threads=1`
  - `cargo test bridge_runtime:: --lib -- --test-threads=1`
  - `cargo test -p sempal --lib -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Plan order deviation: items 4-7 remained blocked on clarification/dependency, so item 8 was executed next as the highest-value safe item.

## Open Questions / Missing Definitions

### [!] 1. Should compare-anchor state participate in undo/redo?

- Evidence:
  - `docs/design_principles.md:126-134` promises uniform undo/redo semantics for meaningful in-session workflows.
  - `src/app/controller/playback/compare_anchor.rs` mutates compare-anchor state directly.
  - `src/app/controller/history.rs` does not currently snapshot or restore compare-anchor fields.
- Why this matters: implementation order for history fixes and tests depends on whether compare-anchor is meant to be transient or undoable.
- Affected files/modules:
  - `src/app/controller/playback/compare_anchor.rs`
  - `src/app/controller/history.rs`
  - `src/app/controller/tests/compare_anchor.rs`
- Risk if guessed incorrectly: undo/redo either remains surprisingly incomplete or silently widens around a state the maintainers intended to keep transient.
- Most conservative provisional assumption: do not widen or narrow the undo contract until compare-anchor semantics are explicitly defined.

### [!] 2. What is the intended lifecycle for unmatched `pending_wav_renames` rows?

- Evidence:
  - Quick scans stage leftover missing rows.
  - Deep scans only clear rows on unique matches.
  - Current tests intentionally allow ambiguous pending rows to remain.
- Why this matters: safe implementation depends on whether the intended outcome is indefinite retention, hard-rescan pruning, bounded retention, or some other policy.
- Affected files/modules:
  - `src/sample_sources/db/pending_renames.rs`
  - `src/sample_sources/scanner/scan_diff.rs`
  - `src/sample_sources/scanner/scan_hash.rs`
  - `src/sample_sources/scanner/scan/tests.rs`
- Risk if guessed incorrectly: either metadata is lost too aggressively or stale rows linger and create surprising future matches.
- Most conservative provisional assumption: keep current behavior unchanged until the intended retention policy is documented.

### [!] 3. Should `GuiScenarioStep::CaptureSnapshot` add labeled intermediate artifacts, or should the step be removed?

- Evidence:
  - The scenario schema exposes the step.
  - The runner currently does nothing for it.
  - The artifact bundle currently stores only one final `automation_snapshot`.
- Why this matters: fixing the no-op requires either an artifact/schema expansion or a contract simplification, and the right split boundary for `src/gui_test/runner.rs` depends on that decision.
- Affected files/modules:
  - `src/gui_test/scenario.rs`
  - `src/gui_test/runner.rs`
  - `src/gui_test/artifacts.rs`
  - `tools/gui-test-cli/src/main.rs`
- Risk if guessed incorrectly: future tooling depends on a misleading no-op contract or the schema grows in a direction the maintainers do not want.
- Most conservative provisional assumption: unsupported capture steps should not remain silently advertised as successful behavior.

### [!] 4. Is `app_core` still expected to narrow legacy state ownership beyond path-level centralization?

- Evidence:
  - `docs/gui_migration_parity.md:82-97` describes `app_core::app_api` and `app_core::state` as the migration-facing boundary.
  - `src/app_core/app_api.rs:32-34` still re-exports the entire legacy `crate::app::state::*` surface.
  - `src/app_core/state.rs` currently narrows some concepts but still aliases many legacy state types directly.
- Why this matters: the minimum safe fix for item 1 is straightforward, but the repo has not fully documented whether deeper type ownership narrowing is still an active goal or just a historical migration note.
- Affected files/modules:
  - `src/app_core/app_api.rs`
  - `src/app_core/state.rs`
  - `docs/gui_migration_parity.md`
- Risk if guessed incorrectly: a small import-path fix could be mistaken for “migration complete,” or a larger narrowing refactor could overreach beyond current repository intent.
- Most conservative provisional assumption: restore the explicit import-path boundary first and treat deeper type-ownership narrowing as separate, future work unless new docs or user direction say otherwise.

## Rejected Ideas

### [-] 1. Broadly redesign the `app_core` migration layer now

- Why it was considered: the migration docs overstate how complete the current boundary cleanup is, and `app_core` still aliases a large legacy surface.
- Why it was rejected: the concrete live failure is a small set of direct `crate::app::` references. A narrow boundary repair is strongly supported by current evidence; a larger redesign is not.
- What evidence was missing: repository-specific proof that a wider migration refactor is currently necessary beyond fixing the broken guardrail.

### [-] 2. Split `src/app_core/actions/catalog/kinds.rs` immediately

- Why it was considered: it remains 555 lines and over the nominal 400-line budget.
- Why it was rejected: `docs/file_size_budget_allowlist.txt` explicitly documents it as an intentional centralized declaration-order surface for action-catalog tooling.
- What evidence was missing: current correctness bugs or ownership pain strong enough to justify breaking that central catalog surface.

### [-] 3. Split `vendor/radiant/src/app/hotkeys.rs` immediately

- Why it was considered: it is 908 lines and houses a large hotkey catalog.
- Why it was rejected: the file is a deliberate single source of truth for shared hotkey bindings, and it already includes uniqueness and resolution tests.
- What evidence was missing: a concrete correctness bug or workflow failure caused by its current shape, rather than file size alone.

### [-] 4. Replace the small custom CLIs with `clap`

- Why it was considered: several workspace tools still parse arguments manually.
- Why it was rejected: the current parsers are small, documented, and the previously missing top-level `gui-test-cli` parse coverage has already been added.
- What evidence was missing: a concrete parser correctness issue or maintenance failure caused by the current approach.

### [-] 5. Promote the desktop-AIV loop into normal CI now

- Why it was considered: the GUI test platform has significant semantic and desktop automation infrastructure.
- Why it was rejected: `docs/gui_test_platform.md` still documents Windows foreground-activation instability as a blocker for CI promotion.
- What evidence was missing: a small stable subset with a documented promotion bar and repeatable success evidence on the current Windows setup.
