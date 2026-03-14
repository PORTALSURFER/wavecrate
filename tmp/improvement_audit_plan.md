# Evidence-Driven Improvement Audit Plan

Generated: 2026-03-14
Repository: `C:\dev\sempal`
Branch: `next`
Phase: 2 in progress
Implementation status: items 1-4 are complete; remaining backlog items are pending sequential execution.

## Repository Context

- Project purpose: Rust desktop audio sample triage and curation tool with native GUI, playback/editing, source management, updater flows, and semantic GUI automation.
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `docs/design_principles.md`, `docs/ARCHITECTURE.md`
- Maturity level: actively developed application with strong local/CI guardrails, but still carrying meaningful migration debt, large-file debt, and some duplicated behavior paths.
  - Confidence: Strongly implied by code/docs
  - Evidence: `.github/workflows/ci.yml`, `docs/QUALITY_SCORE.md`, `docs/gui_migration_parity.md`, `tmp/cleanup_audit_hotspots.md`
- Primary languages/tooling: Rust 2024 workspace, Cargo, PowerShell-first Windows workflow wrappers, `vendor/radiant` native GUI/runtime, semantic GUI test tooling plus AIV desktop automation.
  - Confidence: Explicitly documented
  - Evidence: `Cargo.toml`, `README.md`, `AGENTS.md`, `docs/TEST.md`, `docs/gui_test_platform.md`
- Repository shape: root app crate plus companion apps under `apps/`, support tools under `tools/`, and the `vendor/radiant` submodule.
  - Confidence: Explicitly documented
  - Evidence: `Cargo.toml`, `docs/ARCHITECTURE.md`
- Architectural boundaries: `src/**` owns domain logic, `src/app_core/**` owns backend-neutral projection/action contracts, and `vendor/radiant/**` owns GUI behavior/runtime concerns.
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `docs/ARCHITECTURE.md`
- Test strategy: unit and integration tests first, then GUI contract/snapshot coverage, with AIV desktop automation as a broader live loop.
  - Confidence: Explicitly documented
  - Evidence: `docs/TEST.md`, `docs/gui_test_platform.md`
- Canonical local validation commands on Windows:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `AGENTS.md`, `docs/README.md`, `docs/TEST.md`
- Documented priorities: responsiveness, non-blocking behavior, deterministic interaction, semantic GUI automation, explicit docs, and safe/reversible change paths.
  - Confidence: Explicitly documented
  - Evidence: `docs/design_principles.md`, `docs/gui_test_platform.md`, `AGENTS.md`
- Explicit non-goals: DAW replacement, cloud/social platform behavior, framework rewrite-by-default, or ornamental UX at the expense of responsiveness.
  - Confidence: Explicitly documented
  - Evidence: `docs/design_principles.md`
- Current uncertainty that materially affects safe planning:
  - The browser has both sync and async search/filter pipelines with different test/runtime defaults.
  - The migration docs describe a strict `app_core` boundary, but the Windows enforcement path is still not reliable enough to use as a parity gate.
  - Confidence: Strongly implied by code/docs
  - Evidence: `src/app/controller/library/wavs/browser_pipeline.rs`, `src/app/controller/library/wavs/browser_search.rs`, `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs`, `scripts/check_migration_boundary.ps1`, `docs/gui_migration_parity.md`

## ROI-Ranked Backlog

### [x] 1. Restore usable Windows CI parity for the migration-boundary gate

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters:
  - The repo documents `scripts/ci_local.ps1` as the canonical local CI path, but the current Windows migration-boundary check is not usable as a debugging tool. That blocks the highest-signal validation path and slows every follow-up migration cleanup.
- Evidence:
  - `README.md` and `docs/TEST.md` define `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1` as the canonical parity command.
  - `scripts/ci_local.ps1` invokes `scripts/run_agent_ci_checks.ps1`, which in turn relies on the migration-boundary guard.
  - `scripts/check_migration_boundary.ps1:47-58` calls `Write-Error` before printing collected violations, so the actionable list is skipped under `ErrorActionPreference = "Stop"`.
  - Observed during this audit on 2026-03-14: `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` exits with `Migration boundary check failed...` but does not print the offending matches.
- Recommended change:
  - Make `scripts/check_migration_boundary.ps1` print all collected violations before exiting non-zero.
  - Keep its behavior aligned with the documented allowlist/transitional boundary so the failure is actionable instead of opaque.
- Expected impact:
  - Restores an actionable full local parity gate on Windows.
  - Reduces time spent guessing whether the blocker is a real boundary violation or a script/reporting bug.
- Risks / tradeoffs:
  - Low implementation risk if limited to output/flow control.
  - May expose real remaining boundary debt that still needs follow-up work.
- Dependencies:
  - None.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
- Product clarification required: No
- Completed: 2026-03-14
- Commit: `a1bdd698` (`fix(scripts): make migration boundary failures actionable`)
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1` passed with the new migration-boundary fixture coverage.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1` advanced past the migration-boundary gate and failed later in the unrelated pre-existing `vendor/radiant` test `gui::native_shell::layout_adapter::controls::controls_tests::toolbar_search_field_uses_ratio_width_inside_full_host`.
- Assumptions:
  - The `vendor/radiant` layout test failure is outside this item's change surface because item 1 only changed migration-boundary reporting and its script guardrails.

### [x] 2. Eliminate search-behavior drift between the sync browser pipeline and the async worker pipeline

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - Browser filtering/search/sort behavior exists in two separate implementations. That creates a correctness risk where small libraries and large libraries can produce different visible rows, sort order, or folder-filter semantics.
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline.rs:67-339` implements retained sync visible-row construction, folder acceptance, scoring, and sorting in-process.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs:10-398` independently implements worker-side cache setup, query scoring, folder acceptance, visible-row construction, and sorting.
  - `src/app/controller/library/wavs/browser_search.rs:198-205` dispatches the async job path, while the sync path still rebuilds lists locally.
- Recommended change:
  - Extract the shared query/filter/sort semantics into shared helpers, or add an explicit parity layer that both paths consume.
  - If full helper sharing is too invasive, start with a common fixture-driven parity harness and then collapse the duplicated logic incrementally.
- Expected impact:
  - Lowers the risk of runtime-only browser mismatches.
  - Makes future browser search fixes safer because there is one source of behavior truth or one parity contract.
- Risks / tradeoffs:
  - Medium refactor risk in a central browsing path.
  - Over-abstraction would be a mistake here; the safe version is shared semantics, not a forced giant common framework.
- Dependencies:
  - Item 3 benefits this work but is not required.
- Suggested validation:
  - Deterministic parity tests for query, rating filter, folder filter, similar-query, and playback-age sort combinations.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: 2026-03-14
- Commit: `7a362804` (`fix(browser): align async search ordering`)
- Validation outcome:
  - `cargo test browser_search_worker -- --nocapture` passed.
  - `cargo test browser_core -- --nocapture` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.
- Assumptions:
  - Matching the sync pipeline's existing score-ranked query order for `ListOrder` search results is the safest parity target because the controller tests and retained pipeline already treat fuzzy-score order as the contract.

### [x] 3. Add direct coverage for the runtime-default async browser search path

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - The runtime-default browser search path is explicitly different from the test-default path. Current controller tests mostly validate the synchronous path, so regressions in the actual runtime behavior can slip through.
- Evidence:
  - `src/app/controller/library/wavs/browser_search.rs:69-70` routes browser interactions through `browser_async_pipeline_enabled()`.
  - `src/app/controller/library/wavs/browser_search.rs:407-420` defaults async search to `true` outside tests and `false` under `#[cfg(test)]`.
  - `src/app/controller/tests/browser_core.rs` exercises browser search/filter behavior through the controller-facing sync rebuild path.
  - `src/app/controller/library/wavs/browser_search_worker.rs` has worker-unit coverage, but not end-to-end controller/runtime parity coverage for the default async interaction mode.
- Recommended change:
  - Add a deterministic test harness that forces the async path under test and verifies visible rows, busy-state, request-id, and result-application semantics.
  - Include at least one parity check against the sync path for the same fixture.
- Expected impact:
  - Better confidence in the default runtime browser-search behavior.
  - Faster diagnosis of regressions that only show up with background search jobs enabled.
- Risks / tradeoffs:
  - Test harness work is moderately involved because the worker path is asynchronous.
  - Avoid timing-fragile tests; prefer explicit queue/result injection.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted browser-search tests covering async dispatch and application.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: 2026-03-14
- Commit: `888b47a2` (`test(browser): cover async search controller flow`)
- Validation outcome:
  - `cargo test browser_async -- --nocapture` passed.
  - `cargo test browser_search -- --nocapture` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.
- Assumptions:
  - A deterministic test-only async-dispatch override and direct background-message injection are acceptable seams because they exercise the real controller request-id and busy-state logic without introducing timing-fragile worker waits into the test suite.

### [x] 4. Finish the remaining `app_core` migration-boundary cleanup in controller shims

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - `app_core` is documented as the backend-neutral migration-facing layer, but the current controller shims still depend directly on legacy `crate::app` types and helpers outside the documented bridge surface. That keeps the boundary porous and harder to reason about.
- Evidence:
  - `docs/gui_migration_parity.md:88-89` states CI should fail if `app_core` imports direct legacy `crate::app::` paths outside the bridge boundary.
  - `src/app_core/controller.rs:10` re-exports `crate::app::controller::build_named_gui_fixture_controller`.
  - `src/app_core/controller.rs:322` uses `crate::app::controller::StatusTone::Info`.
  - `src/app_core/controller/waveform_actions.rs:7` imports `crate::app::state::DestructiveSelectionEdit`.
  - `src/app_core/controller/waveform_actions.rs:157` uses `crate::app::controller::StatusTone::Error`.
- Recommended change:
  - Route these remaining legacy types through `app_api`/`legacy_bridge` aliases or a narrower migration-owned shim module.
  - After that cleanup, simplify the transitional exceptions in the migration-boundary check.
- Expected impact:
  - Makes the documented boundary real instead of aspirational.
  - Reduces migration confusion for future `app_core` work.
- Risks / tradeoffs:
  - Moderate churn in migration-facing glue.
  - Must preserve native runtime wiring and tests.
- Dependencies:
  - Item 1 makes this easier to validate locally.
- Suggested validation:
  - `rg -n "crate::app::" src/app_core`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
- Product clarification required: No
- Completed: 2026-03-14
- Commit: `b232cfec` (`refactor(app_core): enforce migration boundary`)
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1` passed.
  - `cargo test app_core::controller -- --nocapture` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.
- Assumptions:
  - The remaining migration-facing controller shims are intended to consume legacy controller/state aliases through `app_core::app_api` rather than through direct `crate::app::` imports, because the guardrail and migration docs already describe `app_api` as the explicit legacy boundary.

### [x] 5. Refresh the stale file-size debt ledger and dependent audit docs

- Classification: Documentation gap
- Confidence: High
- ROI: High
- Effort: S
- Why it matters:
  - The allowlist is one of the repo’s planning inputs. Right now it still names files that no longer exist, so it no longer accurately reflects the live debt and can misdirect cleanup work.
- Evidence:
  - `docs/file_size_budget_allowlist.txt:13` still lists `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs`, which no longer exists in the tree.
  - `docs/file_size_budget_allowlist.txt:17` still lists `src/app/controller/playback/audio_options.rs`, which no longer exists in the tree.
  - `docs/file_size_budget_allowlist.txt:25`, `:30`, and `:46` still list legacy split paths that no longer exist (`src/app_core/native_bridge/tests/projection_cache.rs`, `src/waveform/zoom_cache.rs`, `vendor/radiant/src/gui_runtime/native_vello/input/waveform_routing.rs`).
  - `tmp/cleanup_audit_hotspots.md` reports a different current over-budget set headed by `src/app/controller/tests/browser_core.rs`, `src/app_core/actions/catalog.rs`, `tests/unit/source_db_mod_tests.rs`, and `apps/installer/src/ui.rs`.
- Recommended change:
  - Prune nonexistent allowlist entries.
  - Refresh any linked audit/quality docs that still describe the old debt set.
- Expected impact:
  - Makes the cleanup ledger trustworthy again.
  - Prevents future audit work from targeting already-split or removed files.
- Risks / tradeoffs:
  - Low technical risk.
  - The main risk is forgetting to refresh related docs in the same pass.
- Dependencies:
  - None.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed on: `2026-03-14`
- Commit: `fe44b501`
- Validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.
- Assumptions:
  - `docs/file_size_budget_allowlist.txt` is intended to track only the guardrail scope enforced by `scripts/check_file_size_budget.*` (`src/`, `tests/`, and `vendor/radiant/src`), while `tmp/cleanup_audit_hotspots.md` remains the broader multi-tree hotspot snapshot for `apps/` and `tools/` debt.

### [x] 6. Split the canonical GUI action catalog into smaller contract-focused modules

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The GUI action catalog is a central semantic contract for tooling, automation, and tests. It currently concentrates action identity, coverage metadata, sample payload factories, and exhaustive matching in one large file, which increases review risk whenever the catalog evolves.
- Evidence:
  - `docs/gui_test_platform.md` names `src/app_core/actions/catalog.rs` as the canonical host action catalog.
  - `tmp/cleanup_audit_hotspots.md` lists `src/app_core/actions/catalog.rs` at 595 lines, currently one of the largest production files.
  - `src/app_core/actions/catalog.rs:9-232` defines the large `GuiActionKind` surface.
  - `src/app_core/actions/catalog.rs:429-452` generates both the catalog and representative action payload mapping in one macro block.
  - `src/app_core/actions/tests.rs` verifies exhaustiveness, so this surface is already treated as contract-critical.
- Recommended change:
  - Split action identity, coverage metadata, and representative sample payloads into focused modules while preserving the exhaustive compile-time match path.
- Expected impact:
  - Lower maintenance risk in a repo-wide contract surface.
  - Easier review of future GUI action additions.
- Risks / tradeoffs:
  - Moderate mechanical churn.
  - Preserve the current exhaustiveness guarantees; do not replace them with dynamic lookup magic.
- Dependencies:
  - None.
- Completed on: `2026-03-14`
- Validation:
  - `cargo test app_core::actions -- --nocapture` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.
- Assumptions:
  - The stable public surface should remain the `app_core::actions` re-exports, so splitting the implementation under `src/app_core/actions/catalog/` is preferable to renaming public symbols or changing lookup semantics.
- Suggested validation:
  - `cargo test app_core::actions -- --nocapture`
  - GUI contract loop from `docs/gui_test_platform.md`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 7. Split the installer native bridge into explicit state, task, and projection layers

- Classification: Architecture improvement
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters:
  - The installer is a user-visible state machine that currently mixes background task startup, event polling, step transitions, projection building, and action reduction in one file. That shape is harder to extend safely than the already-split updater-helper flow.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `apps/installer/src/ui.rs` at 470 lines.
  - `apps/installer/src/ui.rs:50-394` keeps `InstallerNativeBridge`, `start_install`, `poll_installer`, `app_model`, and `reduce_action` together in one file.
  - `apps/installer/src/ui.rs:459-467` currently has only a minimal step-order test.
  - `apps/updater-helper/src/ui/{tasks,state,projection}.rs` already shows the repo’s preferred decomposition for a similar native companion UI.
- Recommended change:
  - Mirror the updater-helper split: isolate worker-task plumbing, reducer-like state transitions, and view projection.
  - Add targeted tests for failure recovery, finish actions, and per-step model projection.
- Expected impact:
  - Safer maintenance of the installer flow.
  - Better local reasoning about state transitions and background event handling.
- Risks / tradeoffs:
  - Moderate churn in a smaller app path.
  - Avoid over-engineering; this should stay a small, explicit state machine.
- Dependencies:
  - None.
- Suggested validation:
  - `cargo test -p sempal-installer -- --nocapture`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 8. Deduplicate the rating/tagging mutation and undo/refocus logic in playback tagging

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - Tagging and rating changes are core curation flows, and the current implementation duplicates a large amount of context gathering, mutation, undo wiring, and refocus logic across two long functions. That makes future behavior fixes easy to land in one path but miss in the other.
- Evidence:
  - `src/app/controller/playback/tagging.rs:83-210` implements `tag_selected(...)`.
  - `src/app/controller/playback/tagging.rs:231-420` implements `adjust_selected_rating(...)`.
  - Both functions resolve browser contexts, dedupe paths, apply source mutations, build undo payloads, restore focus, and call `advance_or_commit_after_rating(...)`.
  - `tmp/cleanup_audit_hotspots.md` reports `src/app/controller/playback/tagging.rs` at 424 lines and flags `adjust_selected_rating` as a 194-line function span.
- Recommended change:
  - Extract shared batch-mutation and undo/refocus helpers, keeping rating-specific auto-trash and auto-advance rules as narrow policy layers.
- Expected impact:
  - Lower drift risk in one of the app’s highest-churn user workflows.
  - Smaller diffs when rating/tagging behavior needs adjustment.
- Risks / tradeoffs:
  - Moderate refactor risk in behavior-sensitive controller code.
  - Preserve current user-visible semantics and regression tests.
- Dependencies:
  - None.
- Suggested validation:
  - Existing browser/tagging controller tests in `src/app/controller/tests/browser_core.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 9. Add direct tests and smaller pure seams for the Windows external drag-out implementation

- Classification: Test gap
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters:
  - The Windows drag-out path is unsafe host-integration code built on COM, OLE, and `CF_HDROP`. The controller has dwell/launch tests, but the lower-level payload construction and data-object behavior currently have no direct coverage.
- Evidence:
  - `src/external_drag.rs:79-364` implements `FileDropDataObject`, COM interfaces, and `create_hglobal_for_paths(...)`.
  - `src/external_drag.rs:415-426` normalizes Windows paths, including verbatim-path trimming.
  - `tmp/cleanup_audit_hotspots.md` lists `src/external_drag.rs` at 432 lines and flags it as a likely test-gap hotspot.
  - Existing tests in `src/app/controller/ui/drag_drop_controller/actions/external_drag.rs` cover launch heuristics, not the platform payload format itself.
- Recommended change:
  - Split pure path/payload formatting from COM object plumbing so the payload contract can be unit-tested.
  - Add platform-gated tests for path normalization and `DROPFILES` payload construction.
- Expected impact:
  - Better confidence in a brittle OS integration boundary.
  - Easier diagnosis if drag-out breaks on Windows.
- Risks / tradeoffs:
  - Medium effort because some logic is necessarily Windows-specific.
  - Keep the COM layer thin; the testable seam should stay data-oriented.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted Windows unit tests for drag payload helpers.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 10. Split the remaining monolithic regression catalogs by behavior family

- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters:
  - Several test files remain large enough that adding or reviewing regressions requires touching unrelated behaviors. That slows auditing and raises merge-noise in areas that are already behavior-dense.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists:
    - `src/app/controller/tests/browser_core.rs` at 611 lines
    - `tests/unit/source_db_mod_tests.rs` at 561 lines
    - `src/app/controller/library/analysis_jobs/enqueue/tests.rs` at 543 lines
    - `src/app/controller/tests/focus_random.rs` at 520 lines
  - These files each cover multiple distinct behavior families in one place.
- Recommended change:
  - Split each file by domain slice while preserving existing helper semantics and stable test names where that matters for grep history.
- Expected impact:
  - Easier test discoverability and smaller review surfaces.
  - Lower friction when adding focused regressions during future bug work.
- Risks / tradeoffs:
  - Medium confidence because this is maintainability work, not an immediate correctness fix.
  - Avoid churn for its own sake; only split the clear multi-family catalogs.
- Dependencies:
  - None.
- Suggested validation:
  - Existing targeted cargo test filters for the affected modules.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] What is the intended long-term source of truth for browser search semantics?

- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline.rs` and `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` both implement visible-row semantics.
  - `src/app/controller/library/wavs/browser_search.rs:407-420` makes tests default to sync behavior while runtime defaults to async behavior.
- Why this matters:
  - If the repo intends to keep both paths permanently, then parity must be an explicit invariant.
  - If one path is transitional, the safer recommendation is consolidation rather than long-term dual maintenance.
- Affected files/modules:
  - `src/app/controller/library/wavs/browser_pipeline.rs`
  - `src/app/controller/library/wavs/browser_search.rs`
  - `src/app/controller/library/wavs/browser_search_worker/**`
- Risk if guessed incorrectly:
  - Either duplicated semantics keep drifting, or a premature consolidation removes a deliberate fast path.
- Most conservative provisional assumption:
  - Treat parity as mandatory now; do not remove either path without explicit direction.

### [!] What is the intended end-state of the `app_core` migration boundary exceptions?

- Evidence:
  - `docs/gui_migration_parity.md` describes a strict `app_core` boundary through bridge aliases.
  - `src/app_core/controller.rs` and `src/app_core/controller/waveform_actions.rs` still import `crate::app::` directly.
  - `scripts/check_migration_boundary.ps1` contains transitional exceptions outside the documented `app_api` note.
- Why this matters:
  - The safest cleanup differs depending on whether these direct legacy imports are temporary stragglers or an intentional permanent seam.
- Affected files/modules:
  - `src/app_core/controller.rs`
  - `src/app_core/controller/waveform_actions.rs`
  - `scripts/check_migration_boundary.ps1`
  - `docs/gui_migration_parity.md`
- Risk if guessed incorrectly:
  - Either the code keeps a misleading boundary forever, or a cleanup pass removes a seam that the migration still intentionally depends on.
- Most conservative provisional assumption:
  - Keep the migration seam explicit, but route it through one documented alias boundary rather than scattered direct imports.

### [!] What is the intended division of responsibility between in-process GUI scenarios and desktop AIV packs?

- Evidence:
  - `src/gui_test/scenario.rs` defines in-process declarative scenarios and assertions.
  - `src/gui_test/aiv/mod.rs` defines typed desktop AIV suite manifests.
  - `docs/gui_test_platform.md` lists both as active layers and still calls out desktop AIV stability gaps plus future work.
- Why this matters:
  - Without a documented boundary, new GUI coverage can be added in whichever layer feels convenient, which makes long-term coverage planning harder.
- Affected files/modules:
  - `src/gui_test/scenario.rs`
  - `src/gui_test/aiv/mod.rs`
  - `docs/gui_test_platform.md`
- Risk if guessed incorrectly:
  - Coverage duplication, missing assertions in the right layer, or premature attempts to unify two intentionally different loops.
- Most conservative provisional assumption:
  - Keep semantic in-process scenarios as the deterministic contract layer and desktop AIV as the live-environment validation layer until the boundary is documented more explicitly.

## Rejected Ideas

### [-] Convert browser search to async-only immediately

- Why it was considered:
  - Two search pipelines currently exist.
- Why it was rejected:
  - The repository clearly still supports both paths, and there is not enough evidence that removing the sync path is currently desired.
- Missing evidence:
  - No documented plan to deprecate the in-process path outright.

### [-] Reopen a broad legacy `src/app` cleanup sweep

- Why it was considered:
  - Many large files still live in legacy controller paths.
- Why it was rejected:
  - The repo documents focused migration boundaries and discourages broad speculative rewrites. The evidence supports targeted seams, not a blanket legacy rewrite.
- Missing evidence:
  - No current plan or doc asking for a wide legacy-controller migration sweep.

### [-] Promote desktop AIV into mandatory CI immediately

- Why it was considered:
  - The GUI platform is a major documented investment.
- Why it was rejected:
  - `docs/gui_test_platform.md` explicitly says the current Windows setup still has foreground/focus stability issues and no CI gate yet enforces desktop AIV smoke stability.
- Missing evidence:
  - No repository evidence that the desktop loop is stable enough today to become a required gate.
