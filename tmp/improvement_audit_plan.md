# Improvement Audit Plan

Generated: 2026-04-02
Observed superproject commit: `2205ff4e`
Observed `vendor/radiant` commit: `f6f722ab`
Observed workspace state at audit start: dirty worktree (modified `src/**`, dirty `vendor/radiant`, regenerated `tmp/cleanup_audit_hotspots.md`)
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
- Architectural boundaries: Explicitly documented. `README.md`, `docs/ARCHITECTURE.md`, and `docs/INDEX.md` say `src` owns domain state and UI intent, `vendor/radiant` owns GUI behavior, and `src/app_core/app_api.rs` is the single allowed legacy `crate::app` crossing inside `app_core`.
- Test strategy: Explicitly documented. `docs/TEST.md` and `.github/workflows/ci.yml` center the repo on `devcheck`, `ci_agent`, `ci_quick`, `ci_local`, deterministic Rust unit/integration tests, and the Windows GUI contract wrappers.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/run_agent_request.ps1`, `scripts/check_migration_boundary.ps1`, `scripts/check_file_size_budget.ps1 -All`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/run_gui_contract.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, deterministic interaction, reversibility, non-blocking execution, and data integrity.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, social network, or retention-driven product.
- What the repo appears to be moving toward: Strongly implied by code/docs. The live tree continues to prioritize `app_core` migration-boundary repairs, file-size debt burn-down through focused splits, and GUI automation contract hardening over broad renderer/controller redesigns.
- What is merely possible but unsupported: Weakly implied / uncertain. A one-step shared action-id source across `src/app_core` and `vendor/radiant`, a broad legacy-controller extraction, or immediate desktop-AIV CI promotion are not justified by the current repository evidence.

## Audit Baseline

- `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` currently fails during the mandatory preflight because `src/app_core/**` contains direct `crate::app::` references outside `src/app_core/app_api.rs`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` currently fails on four non-allowlisted files:
  - `src/app/controller/tests/browser_core/marks.rs` (`514`)
  - `src/app/controller/tests/waveform_nav_render.rs` (`414`)
  - `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs` (`408`)
  - `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome/folders.rs` (`421`)
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md` on `2026-04-02`; the live full scan now shows `12` over-budget Rust files total, with `8` documented allowlist exceptions and the `4` live non-allowlisted regressions above.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` currently downgrades the quality score to `3` because the file-size budget check is red.
- Long-form docs currently lag the live guardrail state:
  - `docs/QUALITY_SCORE.md` still says the full-scan file-size budget is green.
  - `docs/gui_migration_parity.md` still lists older playback-age migration blockers instead of the current folder-pane `app_core` violations.
- Other high-visibility guardrails are currently green on the live tree:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_legacy_app_coupling.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_app_core_dependency_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_rust_public_docs.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_rust_no_todos.ps1`

## Ordered Backlog

### 1. [x] Restore the `app_core` migration boundary for folder-pane state types

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the repository’s mandatory preflight is red because `app_core` is bypassing its documented single legacy crossing point, so the current live tree no longer satisfies one of its own enforced migration rules.
- Evidence:
  - `scripts/check_migration_boundary.ps1` allows direct `crate::app::` references only in `src/app_core/app_api.rs`.
  - `docs/ARCHITECTURE.md` and `docs/INDEX.md` repeat the same invariant and remediation path.
  - `src/app_core/app_api.rs` already re-exports `crate::app::state::*` under `crate::app_core::app_api::state`.
  - Current violations are limited to folder-pane state references in:
    - `src/app_core/controller/browser_actions/browser.rs`
    - `src/app_core/controller/browser_actions/folders.rs`
    - `src/app_core/controller/waveform_actions/selection.rs`
    - `src/app_core/native_shell/sources_projection.rs`
- Recommended change: route the flagged `FolderPaneId` and `FolderBrowserUiState` imports/annotations back through `crate::app_core::app_api::state`, keep behavior unchanged, and refresh the stale blocker note in `docs/gui_migration_parity.md` once the live checker is green.
- Expected impact: restores `scripts/check_migration_boundary.ps1` and the required `scripts/run_agent_request.ps1` preflight without widening the allowed transitional boundary.
- Risks / tradeoffs: Low. This is a narrow seam repair, but it does not settle the broader long-term migration shape beyond the path-level boundary.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: `2026-04-02`
- Commit hash: `222eda75` (`fix(app-core): restore folder-pane migration boundary`)
- Assumption used: `crate::app_core::app_api::state` remains the intended migration-facing alias surface for `FolderPaneId` and `FolderBrowserUiState`, so routing those four callers through it preserves the documented boundary without changing behavior.
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` ✅
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` ✅
- Deviation from original plan order: none

### 2. [ ] Split drop-target transfer orchestration to clear the production file-size regression

- Classification: Refactor / cleanup
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: one of the four live non-allowlisted size regressions sits in active drag/drop transfer code, and the file currently mixes controller entrypoints, request planning, worker launch, and destination-path rules in one production module.
- Evidence:
  - `scripts/check_file_size_budget.ps1 -All` reports `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs` at `408` lines.
  - The file already delegates to sibling modules `apply_result`, `transactions`, and `worker`, but still keeps distinct concerns together:
    - controller entrypoints: `handle_sample_drop_to_drop_target`, `handle_samples_drop_to_drop_target`, `handle_samples_transfer_to_source_folder`
    - planning/job helpers: `collect_drop_target_transfer_requests`, `cached_drop_target_metadata`, `spawn_drop_target_transfer_job`, `finish_drop_target_transfer_job`
    - destination helpers: `move_destination_relative`, `copy_destination_relative`, `progress_title`
  - The surrounding `drag_effects/` tree already uses more focused decomposition for neighboring flows such as `source_moves/` and `folder_moves/`.
- Recommended change: keep the public controller entrypoints in a thin coordinating module and extract request/planning helpers and destination-path helpers into focused siblings under the existing `drop_targets/` tree.
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
- ROI: High
- Effort: M
- Why it matters: the remaining `vendor/radiant` production size regression sits in folder sidebar hit-testing, where one file now mixes row targeting, inline editor geometry, scrollbar math, and header toggle dispatch across runtime and test call sites.
- Evidence:
  - `scripts/check_file_size_budget.ps1 -All` reports `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome/folders.rs` at `421` lines.
  - `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome.rs` already fans the chrome hit-testing area into dedicated siblings (`top_bar.rs`, `source_controls.rs`, `prompts.rs`), so the current monolithic folder file is inconsistent with the surrounding structure.
  - `folders.rs` currently combines distinct responsibility clusters inside one `impl NativeShellState`:
    - row/panel/disclosure hit-testing
    - inline folder-create / rename geometry
    - scrollbar/viewport math
    - header toggle hit-testing and test rect helpers
- Recommended change: split the file into focused sibling modules such as row hit-testing, inline editor geometry, scrollbar behavior, and header actions while preserving the current `NativeShellState` API shape where possible.
- Expected impact: clears the live `vendor/radiant` production size regression and makes one of the most geometry-heavy sidebar paths easier to audit and test.
- Risks / tradeoffs: Medium. Pointer hit-testing and viewport behavior are user-visible and easy to regress if helper moves accidentally change shared state access.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted `vendor/radiant` sidebar and folder-toggle tests
  - targeted runtime pointer/viewport tests touching folder hit-testing
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 4. [ ] Split `waveform_nav_render.rs` by render-meta versus async load behavior

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: S
- Why it matters: the saved audit lane missed this new live file-size regression, and the current test hub already mixes distinct waveform concerns that have clearer homes in the existing test tree.
- Evidence:
  - `scripts/check_file_size_budget.ps1 -All` reports `src/app/controller/tests/waveform_nav_render.rs` at `414` lines.
  - The file currently mixes three behavior groups:
    - render-meta matching and viewport-width tests (`waveform_refresh_respects_view_slice_and_caps_width`, `waveform_render_meta_*`)
    - pan/texture stability tests (`adjacent_pan_translation_matches_full_render_output`, `waveform_texture_width_is_stable_for_adjacent_sizes`)
    - async load/playback state tests (`stale_audio_results_are_ignored`, `play_request_is_deferred_until_audio_ready`, `loading_flag_clears_after_audio_load`, `queue_audio_load_failure_clears_loading_state`)
  - `src/app/controller/tests/mod.rs` already separates neighboring waveform concerns into `waveform`, `waveform_cache_loading`, and `waveform_nav_cursor`, which is concrete evidence that the current hub can be decomposed without inventing a new test taxonomy.
- Recommended change: split `waveform_nav_render.rs` into subject-focused sibling modules, or move the async loading cases into the existing waveform-loading test area if that preserves the current test vocabulary more cleanly.
- Expected impact: restores the file-size budget for this new regression and improves discoverability of waveform-render versus load-state failures.
- Risks / tradeoffs: Low. This is structural test cleanup, but moving cases between nearby modules can make fixture ownership slightly less obvious if the split is not named carefully.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted waveform render/loading controller tests
  - `cargo test -p sempal waveform -- --test-threads=1`
- Product clarification required: No

### 5. [ ] Split the oversized browser-mark test hub by behavior family

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: S-M
- Why it matters: the full-scan file-size budget will remain red unless the remaining browser-mark regression file is decomposed, and the current 500-line file already contains clearly separable behavior groups.
- Evidence:
  - `scripts/check_file_size_budget.ps1 -All` reports `src/app/controller/tests/browser_core/marks.rs` at `514` lines.
  - `src/app/controller/tests/browser_core/mod.rs` already splits the rest of browser-core coverage into focused siblings (`filters`, `loading`, `selection`, `tagging`), leaving `marks.rs` as the unusually large remaining hub.
  - The file currently mixes multiple mark-specific families:
    - basic marking semantics
    - preview/autoadvance behavior
    - random-navigation follow-up behavior
    - filter/source-switch persistence behavior
  - Each family is already expressed as separate test functions with minimal cross-test coupling.
- Recommended change: split `marks.rs` into behavior-grouped sibling test modules so the size budget is restored without allowlisting a file that already has natural seams.
- Expected impact: restores the file-size budget, improves test discoverability, and makes future mark-related failures easier to localize.
- Risks / tradeoffs: Low. This is structural test cleanup, but fixture/helper imports still need to stay readable.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted browser-core mark coverage
  - `cargo test -p sempal browser_core:: -- --test-threads=1`
- Product clarification required: No

### 6. [ ] Strengthen automation action-id parity checks where the native shell still hardcodes stable action strings

- Classification: Test gap
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters: the GUI test platform documents the host action catalog as canonical, but the native-shell automation snapshot still builds many advertised action ids through a separate matcher and hardcoded strings, so a control can drift to a wrong-but-still-cataloged id without tripping the current tests.
- Evidence:
  - `docs/gui_test_platform.md` says every `UiAction` should have a host-owned catalog entry and explicitly calls the host catalog canonical.
  - `src/app_core/actions/catalog/entries.rs` defines the stable host `GUI_ACTION_CATALOG`.
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

### 7. [ ] Refresh long-form guardrail docs that now contradict the live tree

- Classification: Documentation gap
- Confidence: High
- ROI: Medium-Low
- Effort: S
- Why it matters: future agents and maintainers use these docs for orientation, but two of the most visible status docs currently describe a greener state than the live guardrails report.
- Evidence:
  - `docs/QUALITY_SCORE.md` says the full-scan file-size budget is green, while `scripts/check_file_size_budget.ps1 -All` and `scripts/check_quality_score_drift.ps1` show it is currently red.
  - `docs/gui_migration_parity.md` still names older playback-age files as the active migration blockers, while the live checker now flags folder-pane references in `src/app_core/controller/browser_actions/browser.rs`, `folders.rs`, `waveform_actions/selection.rs`, and `native_shell/sources_projection.rs`.
  - The repo’s wake-up path explicitly tells agents to read these docs before judging the codebase.
- Recommended change: update the stale blocker/status prose to reflect the current tree, and prefer pointing readers at live guardrail sources (`scripts/check_*`, `tmp/cleanup_audit_hotspots.md`, `tmp/improvement_audit_plan.md`) when exact counts or file lists are likely to rot quickly.
- Expected impact: reduces wake-up drift and keeps future audits/implementations aligned with the same authoritative guardrail sources used by CI and local preflight.
- Risks / tradeoffs: Low. The only risk is spending time polishing docs before the underlying code issues are repaired; keep this lane strictly factual.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. Should stable action identifiers remain duplicated across the host catalog and the `vendor/radiant` automation layer, or is a shared lower-layer source intended long term?

- Evidence:
  - `docs/gui_test_platform.md` calls the host catalog canonical.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs` maintains a manual `action_slug` matcher.
  - Several automation builders still emit manual `available_actions` string literals.
  - `src/gui_test/runner/tests.rs` verifies only that advertised ids are cataloged, not that every node’s advertised id is sourced consistently.
- Why this matters: item 6 can safely strengthen parity tests either way, but any deeper deduplication/refactor depends on whether the repo wants a shared action-id source or a boundary-preserving duplicate representation with better tests.
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

### [!] 3. Which artifacts should future agents treat as authoritative when long-form status docs lag the live guardrail state?

- Evidence:
  - `docs/gui_migration_parity.md` still points to older blocker files that no longer match `scripts/check_migration_boundary.ps1`.
  - `docs/QUALITY_SCORE.md` still describes a green full-scan file-size budget while the live scripts report four non-allowlisted violations and a downgraded score.
  - `AGENTS.md` and `docs/README.md` both route future agents through these docs during wake-up.
- Why this matters: stale status docs can send future agents toward the wrong files or make them report an inaccurately healthy baseline before they inspect the live guardrails.
- Affected files/modules:
  - `docs/gui_migration_parity.md`
  - `docs/QUALITY_SCORE.md`
  - `AGENTS.md`
  - `docs/README.md`
  - `tmp/improvement_audit_plan.md`
- Risk if guessed incorrectly: future work can prioritize the wrong blocker set, understate current debt, or duplicate already-completed analysis.
- Most conservative provisional assumption: treat the live guardrail scripts and generated snapshots (`scripts/check_*`, `tmp/cleanup_audit_hotspots.md`, and this audit plan) as authoritative until the long-form docs are refreshed.

## Rejected Ideas

### [-] 1. Allowlist the four current over-budget files instead of splitting them

- Why it was considered: `scripts/check_file_size_budget.ps1 -All` is currently red on those files.
- Why it was rejected: each file has a clear responsibility split already visible in the code, while the existing allowlist is documented as a home for intentional cohesive exceptions rather than active production/test regressions with natural boundaries.
- What evidence was missing: any cohesion note or explicit architectural justification for keeping those files oversized.

### [-] 2. Weaken `scripts/check_migration_boundary.*` by adding more allowed transitional files

- Why it was considered: the current preflight failure is limited to a few files and symbols.
- Why it was rejected: `src/app_core/app_api.rs` already exposes the required legacy state types, so broadening the allowlist would hide accidental drift instead of repairing the documented invariant.
- What evidence was missing: any repository documentation saying the single-crossing-point rule had been intentionally relaxed.

### [-] 3. Collapse the host catalog and vendor automation action-id generation into one shared source immediately

- Why it was considered: the current automation layer duplicates stable action ids through `action_slug` and manual string literals.
- Why it was rejected: the repository documents the host catalog as canonical but does not yet clearly document whether a shared lower-layer source is desired or boundary-safe.
- What evidence was missing: an explicit ownership decision for where shared action-id derivation should live across `src/app_core` and `vendor/radiant`.

### [-] 4. Treat the stale migration/quality docs as harmless historical context and skip refreshing them

- Why it was considered: the live guardrail scripts already provide the authoritative machine-checked state.
- Why it was rejected: the wake-up flow explicitly tells agents to read these docs first, so stale status prose actively increases orientation drift.
- What evidence was missing: any note in those docs saying they are intentionally historical snapshots rather than current guidance.
