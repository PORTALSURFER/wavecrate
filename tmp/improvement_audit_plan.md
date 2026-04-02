# Improvement Audit Plan

Generated: 2026-04-02
Observed superproject commit: `dad1b568`
Observed `vendor/radiant` commit: `e11f0925`
Observed workspace state at audit start: dirty worktree (`MEMORY.md` refreshed by `scripts/run_agent_request.ps1`)
Status: Phase 1 complete on `2026-04-02`; awaiting explicit user confirmation before any implementation.

## Scope

- This plan audits the current live tree only.
- Findings are ranked in strict execution order by expected ROI for the current repository state.
- Recommendations stay inside documented or strongly implied repository intent.
- No implementation was performed during this audit.
- Broad rewrites, speculative features, and preference-only cleanup are excluded.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as an early-alpha Rust desktop tool for triaging, auditioning, editing, and curating local audio samples with a listening-first workflow.
- Maturity level: Explicitly documented. `README.md` warns that the app is early alpha and can modify, rename, or delete library files.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace centered on the root `sempal` crate, companion apps/tools, and the vendored `radiant` GUI/runtime crate.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` splits domain/controller logic across `src/`, migration-facing projections/actions under `src/app_core`, runtime bridge code under `src/gui_runtime`, GUI behavior under `vendor/radiant`, and support tooling under `apps/` and `tools/`.
- Architectural boundaries: Explicitly documented. `docs/ARCHITECTURE.md` and `docs/INDEX.md` say `src` owns domain state and UI intent, `vendor/radiant` owns GUI behavior, and `src/app_core/app_api.rs` is the single allowed legacy `crate::app` crossing inside `app_core`.
- Test strategy: Explicitly documented. `docs/TEST.md` and `.github/workflows/ci.yml` center the repo on `devcheck`, `ci_agent`, `ci_quick`, `ci_local`, deterministic Rust unit/integration tests, and the Windows GUI contract wrappers.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/run_agent_request.ps1`, `scripts/check_migration_boundary.ps1`, `scripts/check_file_size_budget.ps1 -All`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/run_gui_contract.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, deterministic interaction, non-blocking execution, reversibility, and data integrity.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, social network, or retention-driven product.
- What the repo appears to be moving toward: Strongly implied by code/docs. The live tree continues to prioritize `app_core` migration guardrails, GUI automation/catalog coverage, and behavior-preserving cleanup over large renderer/controller redesigns.
- What is merely possible but unsupported: Weakly implied / uncertain. A broad legacy-controller extraction, immediate desktop-AIV CI promotion, or a one-step shared host/vendor action-id source are not justified by the current repository evidence.

## Audit Baseline

- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` currently fails during the mandatory preflight because `scripts/check_migration_boundary.ps1` finds direct `crate::app::` references outside `src/app_core/app_api.rs`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` currently fails on three non-allowlisted files:
  - `src/app/controller/tests/browser_core/marks.rs` (`514`)
  - `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs` (`408`)
  - `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome/folders.rs` (`410`)
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` currently downgrades the score to `3` because the file-size budget check is red.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md` on `2026-04-02`; the live full scan now shows `11` over-budget files total, with `8` documented allowlist exceptions and the `3` live non-allowlisted regressions above.
- Other high-visibility guardrails are currently green on the live tree:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_legacy_app_coupling.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_app_core_dependency_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_rust_public_docs.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_rust_no_todos.ps1`

## Ordered Backlog

### 1. [ ] Restore the `app_core` migration boundary for folder-pane state types

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the repository’s mandatory preflight is red because `app_core` is bypassing its documented single legacy crossing point, so the current live tree no longer satisfies one of its own enforced migration rules.
- Evidence:
  - `src/app_core/app_api.rs` already centralizes the allowed legacy crossing and re-exports `crate::app::state::*` under `crate::app_core::app_api::state`.
  - `scripts/check_migration_boundary.ps1` allows direct `crate::app::` references only in `src/app_core/app_api.rs`.
  - Current violations are limited to:
    - `src/app_core/controller/browser_actions/browser.rs`
    - `src/app_core/controller/browser_actions/folders.rs`
    - `src/app_core/controller/waveform_actions/selection.rs`
    - `src/app_core/native_shell/sources_projection.rs`
  - The flagged references are path-level imports/annotations for `FolderPaneId` and `FolderBrowserUiState`, not evidence of missing boundary types.
- Recommended change: replace the direct `crate::app::state::...` imports and annotations in the flagged files with the existing `crate::app_core::app_api::state::...` re-exports, and keep the behavior unchanged.
- Expected impact: restores `scripts/check_migration_boundary.ps1` and the required `scripts/run_agent_request.ps1` preflight without widening the allowed transitional boundary.
- Risks / tradeoffs: Low. This is a narrow seam repair, but it does not address the broader long-term dependence on legacy `AppController` methods and fields.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 2. [ ] Split drop-target transfer orchestration to clear the production file-size regression

- Classification: Refactor / cleanup
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: one of the three live non-allowlisted size regressions sits in active drag/drop transfer code, and the file currently mixes controller entrypoints, request planning, worker launch, and destination-path rules in one production module.
- Evidence:
  - `scripts/check_file_size_budget.ps1 -All` reports `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs` at `408` lines.
  - The file already delegates to sibling modules `apply_result`, `transactions`, and `worker`, but still keeps distinct concerns together:
    - controller entrypoints: `handle_sample_drop_to_drop_target`, `handle_samples_drop_to_drop_target`, `handle_samples_transfer_to_source_folder`
    - planning/job helpers: `collect_drop_target_transfer_requests`, `cached_drop_target_metadata`, `spawn_drop_target_transfer_job`, `finish_drop_target_transfer_job`
    - destination helpers: `move_destination_relative`, `copy_destination_relative`, `progress_title`
  - `worker.rs` depends on the destination helpers, which is a concrete sign that those helpers have their own reusable boundary.
- Recommended change: keep the public controller entrypoints in a thin coordinating module and extract request/planning helpers and destination-path helpers into focused sibling modules under the existing `drop_targets/` tree.
- Expected impact: clears one live production guardrail failure and makes cross-source copy/move behavior easier to reason about without changing the existing worker pipeline.
- Risks / tradeoffs: Medium. File-op flows touch copy, move, progress, and error reporting paths, so careless movement could disturb runtime behavior.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted drop-target transfer coverage under `src/app/controller/tests/drag_drop_drop_targets/transfer/`
  - targeted worker coverage under `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets/worker/tests.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 3. [ ] Split folder chrome hit-testing into row/editor/scrollbar/header seams

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: the remaining `vendor/radiant` production size regression sits in folder sidebar hit-testing, where one file now mixes row targeting, inline editor geometry, scrollbar math, and header toggle dispatch across runtime and test call sites.
- Evidence:
  - `scripts/check_file_size_budget.ps1 -All` reports `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome/folders.rs` at `410` lines.
  - `folders.rs` currently combines distinct responsibility clusters inside one `impl NativeShellState`:
    - row/panel/disclosure hit-testing
    - inline folder-create / rename geometry
    - scrollbar/viewport math
    - header toggle hit-testing and test rect helpers
  - Those helpers are consumed from multiple surfaces, including runtime input (`vendor/radiant/src/gui_runtime/native_vello/input/pointer.rs`, `vendor/radiant/src/gui_runtime/native_vello/runtime_input/viewport.rs`) and native-shell tests.
- Recommended change: split the file into focused sibling modules such as row hit-testing, inline editor geometry, scrollbar behavior, and header actions while preserving the current `NativeShellState` API shape where possible.
- Expected impact: clears the last live `vendor/radiant` production size regression and makes one of the most geometry-heavy sidebar paths easier to audit and test.
- Risks / tradeoffs: Medium. Pointer hit-testing and viewport behavior are user-visible and easy to regress if helper moves accidentally change shared state access.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted `vendor/radiant` sidebar and folder-toggle tests
  - targeted runtime pointer/viewport tests touching folder hit-testing
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 4. [ ] Split the oversized browser-mark test hub by behavior family

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: S-M
- Why it matters: the full-scan file-size budget will remain red even after the two production splits unless the remaining `browser_core` test hub is decomposed, and the current 500-line file already contains clearly separable behavior groups.
- Evidence:
  - `scripts/check_file_size_budget.ps1 -All` reports `src/app/controller/tests/browser_core/marks.rs` at `514` lines.
  - The file currently mixes multiple test families:
    - basic marking semantics
    - preview/autoadvance behavior
    - random-navigation follow-up behavior
    - filter/persistence behavior
  - Each family is already expressed as separate test functions with minimal cross-test coupling.
- Recommended change: split `marks.rs` into behavior-grouped sibling test modules so the size budget is restored without allowlisting a file that already has natural seams.
- Expected impact: restores the full-scan file-size budget, improves test discoverability, and makes future mark-related failures easier to localize.
- Risks / tradeoffs: Low. This is structural test cleanup, but fixture/helper imports still need to stay readable.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted browser-core test coverage for the moved mark cases
  - `cargo test -p sempal browser_core:: -- --test-threads=1`
- Product clarification required: No

### 5. [ ] Strengthen automation action-id parity checks where the native shell still hardcodes stable action strings

- Classification: Test gap
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters: the GUI test platform documents the host action catalog as canonical, but the native-shell automation snapshot still builds many advertised action ids through a separate matcher and hardcoded strings, so a control can drift to a wrong-but-still-cataloged id without tripping the current tests.
- Evidence:
  - `docs/gui_test_platform.md` says every `UiAction` should have a host-owned catalog entry and explicitly calls the host catalog canonical.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs` maintains a large standalone `action_slug(&UiAction)` matcher.
  - `vendor/radiant/src/gui/native_shell/state/automation/browser.rs`, `sidebar.rs`, `waveform.rs`, and `dialogs.rs` still hardcode `available_actions` string literals for several nodes.
  - `src/gui_test/runner/tests.rs` only checks that advertised action ids are cataloged, not that every advertised control id matches the control’s routed action semantics.
- Recommended change: add stronger parity tests around automation-advertised action ids and their intended control paths, and defer any shared-source refactor unless it can be done without violating the current host/vendor ownership boundary.
- Expected impact: catches semantic automation drift earlier and reduces the chance of AIV or in-process GUI scenarios targeting a valid but wrong action id.
- Risks / tradeoffs: Medium. Test-only strengthening is safe; deeper deduplication may need an explicit ownership decision first.
- Dependencies: none
- Suggested validation:
  - targeted `src/gui_test/runner/tests.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No, if scoped to parity tests only

## Open Questions / Missing Definitions

### [!] 1. Should stable action identifiers remain duplicated across the host catalog and the `vendor/radiant` automation layer, or is a shared lower-layer source intended long term?

- Evidence:
  - `docs/gui_test_platform.md` calls the host catalog canonical.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs` maintains a manual `action_slug` matcher.
  - Several automation builders still emit manual `available_actions` string literals.
  - `src/gui_test/runner/tests.rs` verifies only that advertised ids are cataloged, not that every node’s advertised id is sourced consistently.
- Why this matters: item 5 can safely strengthen parity tests either way, but any deeper deduplication/refactor depends on whether the repo wants a shared action-id source or a boundary-preserving duplicate representation with better tests.
- Affected files/modules:
  - `docs/gui_test_platform.md`
  - `src/app_core/actions/catalog/`
  - `src/gui_test/runner/tests.rs`
  - `vendor/radiant/src/gui/native_shell/state/automation/`
- Risk if guessed incorrectly: a cleanup could either violate the intended `src` versus `vendor/radiant` boundary or preserve duplication where the project actually wanted one source of truth.
- Most conservative provisional assumption: keep the host catalog canonical, preserve the current vendor-owned snapshot emission path, and add stronger parity tests before attempting any shared-source refactor.

### [!] 2. How aggressively should `app_core` reduce legacy `AppController` and state-layout dependence beyond the path-level `app_api` rule?

- Evidence:
  - `src/app_core/app_api.rs` re-exports the legacy controller and state surface for migration-facing code.
  - The live migration-boundary failure is only a path-level bypass, but the affected files still depend on legacy controller methods and state layout through the allowed `app_api` surface.
  - `docs/ARCHITECTURE.md` continues to describe `app_core` as a migration-facing boundary rather than the final home of legacy controller behavior.
- Why this matters: item 1 is a small, safe repair, but the repo will keep facing migration choices about when path-level fixes are enough versus when a deeper controller seam should be extracted.
- Affected files/modules:
  - `src/app_core/app_api.rs`
  - `src/app_core/controller/`
  - `src/app_core/native_shell/`
- Risk if guessed incorrectly: future work could either widen incidental coupling under the guise of migration or reopen a broad legacy-controller rewrite that the current evidence does not justify.
- Most conservative provisional assumption: repair the direct `crate::app::` path violations now, but defer broader controller extraction to a separate migration-focused lane with its own evidence and scope.

## Rejected Ideas

### [-] 1. Allowlist the three current over-budget files instead of splitting them

- Why it was considered: `scripts/check_file_size_budget.ps1 -All` is currently red on those files.
- Why it was rejected: each file has a clear responsibility split already visible in the code, while the existing allowlist is documented as a home for intentional cohesive exceptions rather than active production/test regressions with natural boundaries.
- What evidence was missing: any cohesion note or explicit architectural justification for keeping those files oversized.

### [-] 2. Weaken `scripts/check_migration_boundary.*` by adding more allowed transitional files

- Why it was considered: the current preflight failure is limited to four files.
- Why it was rejected: `src/app_core/app_api.rs` already exposes the required legacy state types, so broadening the allowlist would hide accidental drift instead of repairing the documented invariant.
- What evidence was missing: any repository documentation saying the single-crossing-point rule had been intentionally relaxed.

### [-] 3. Collapse the host catalog and vendor automation action-id generation into one shared source immediately

- Why it was considered: the current automation layer duplicates stable action ids through `action_slug` and manual string literals.
- Why it was rejected: the repository documents the host catalog as canonical but does not yet clearly document whether a shared lower-layer source is desired or boundary-safe.
- What evidence was missing: an explicit ownership decision for where shared action-id derivation should live across `src/app_core` and `vendor/radiant`.
