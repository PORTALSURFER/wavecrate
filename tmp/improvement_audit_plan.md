# Improvement Audit Plan

Generated: 2026-03-31
Observed superproject commit: `4c99a8c5`
Observed `vendor/radiant` commit: `ff9ae90b`
Observed workspace state at audit start: dirty worktree (`tmp/improvement_audit_plan.md`, dirty `vendor/radiant` submodule)
Status: Phase 2 active on `2026-03-31`. Item 1 is complete; item 2 is next in sequence.

## Scope

- This plan audits the current live tree only.
- Findings are ranked in strict execution order by expected ROI for the current repository state.
- Recommendations stay inside documented or strongly implied repository intent.
- Broad rewrites, speculative features, and preference-only cleanup are excluded.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as an early-alpha Rust desktop tool for triaging, auditioning, editing, and curating local audio samples.
- Maturity level: Explicitly documented. `README.md` warns that the app is early alpha and can modify, rename, or delete library files.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace with the root `sempal` crate, companion apps/tools, and the vendored `radiant` GUI/runtime submodule.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` routes domain/controller logic through `src/`, migration-facing projection/runtime glue through `src/app_core` and `src/gui_runtime`, GUI/runtime behavior through `vendor/radiant/`, and support tooling through `apps/` and `tools/`.
- Architectural boundaries: Explicitly documented. `docs/ARCHITECTURE.md` says domain state and UI intent belong in `src`, while `vendor/radiant` owns widget behavior, layout, hit testing, input routing, and rendering coordination.
- Test strategy: Strongly implied by code/docs. `docs/TEST.md` and `.github/workflows/ci.yml` center the repo on deterministic Rust unit tests, `cargo nextest`, focused GUI contract tests, and optional local-only desktop-AIV loops.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, `scripts/ci_local.ps1`, and GUI-specific wrappers such as `scripts/run_gui_contract.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, non-blocking execution, predictability, reversibility, and data integrity.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, social network, or attention-retention product.

## Audit Baseline

- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` currently fails with `20` non-allowlisted over-budget Rust files.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` currently passes with a downgraded guardrail score of `3`, so the repo recognizes degraded guardrails but not every stale detail inside `docs/QUALITY_SCORE.md`.
- `tmp/cleanup_audit_hotspots.md` reports `27` total over-budget Rust files, `3` files with `dead_code` suppressions, and `3` files with `clippy::too_many_arguments` suppressions.
- The cleanup-hotspot snapshot currently contains at least two heuristic false positives in its “Likely test-gap hotspots” section:
  - `src/analysis/ann_index_tests.rs` is itself a test file.
  - `src/selection/range.rs` is backed by `src/selection/mod.rs` and `src/selection/tests.rs`.
- Mutable handoff docs are currently inconsistent:
  - `AGENTS.md`, `MEMORY.md`, `docs/plans/index.md`, and `docs/plans/active/todo.md` describe Phase 2 as active with item 6 next.
  - `docs/README.md` says the same lane’s Phase 2 is complete.

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for listening to, navigating, editing, and curating local sample libraries with strong emphasis on responsiveness and reversible workflows.
- What the repo appears to be moving toward: Strongly implied by code/docs. More truthful audit tooling, smaller migration/runtime surfaces, deterministic GUI automation, and continued file-size discipline without reopening broad runtime rewrites.
- What is merely possible but unsupported: promoting desktop AIV into CI now, forcing all centralized tables to split, or introducing new product features not already implied by the existing sample-triage workflow.

## Ordered Backlog

### 1. [x] Fix the cleanup-hotspot audit heuristic so it stops misclassifying tested modules as “test-gap” hotspots

- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: this repository now relies on `tmp/cleanup_audit_hotspots.md` for ROI planning, but the current heuristic can overstate missing-test risk and skew cleanup priorities.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/analysis/ann_index_tests.rs` and `src/selection/range.rs` under “Likely test-gap hotspots”.
  - `scripts/audit_cleanup_hotspots.ps1` only skips `*_test.rs`, `tests.rs`, and `tests/**`, so it misses `*_tests.rs`.
  - `scripts/audit_cleanup_hotspots.ps1` only looks for `#[cfg(test)]` or `mod tests` inside the same file.
  - `src/selection/mod.rs` already declares `mod tests;`, and `src/selection/tests.rs` exists.
- Recommended change: teach the audit script to recognize `*_tests.rs` as dedicated test files and to treat module-level sibling tests (`mod.rs` plus `tests.rs`) as real local coverage.
- Expected impact: makes future audit snapshots more trustworthy and prevents false-positive planning churn.
- Risks / tradeoffs: low. This should only narrow false positives; it does not weaken the file-size scan itself.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
  - targeted script-guardrail coverage for `*_tests.rs` and sibling `mod.rs` / `tests.rs` layouts
- Product clarification required: No
- Execution record (`2026-03-31`):
  - Outcome: updated `scripts/audit_cleanup_hotspots.ps1` and `scripts/audit_cleanup_hotspots.sh` to recognize `*_tests.rs` and sibling module coverage declared through `mod.rs` plus `tests.rs`, and added matching fixture coverage in `scripts/check_script_guardrails.ps1` and `scripts/check_script_guardrails.sh`.
  - Assumption used: when a module directory contains both `mod.rs` with local test markers and a sibling `tests.rs`, that layout counts as local coverage for sibling files in the same module.
  - Validation outcome: `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` all passed; the regenerated `tmp/cleanup_audit_hotspots.md` no longer lists `src/analysis/ann_index_tests.rs` or `src/selection/range.rs` in the heuristic test-gap section.
  - Commit note: the item is committed as one focused Conventional Commit in Git history for this lane.

### 2. [ ] Re-synchronize mutable lane-state docs and add one lightweight consistency guard or single source of truth

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S-M
- Why it matters: Sempal explicitly depends on stateless-agent handoff docs, so conflicting phase/status text can send future sessions down the wrong lane before code inspection even starts.
- Evidence:
  - `AGENTS.md`, `MEMORY.md`, `docs/plans/index.md`, and `docs/plans/active/todo.md` describe the improvement-audit Phase 2 as active with item 6 next.
  - `docs/README.md` says the same lane’s “Phase 2 is complete”.
  - `docs/README.md` is part of the documented wake-up path in `AGENTS.md`.
  - Current automated doc checks (`scripts/check_docs_index.*`) validate references, not cross-file lane-state consistency.
- Recommended change: either designate one mutable file as the canonical lane-status source and have the others point at it, or add a small consistency check for the duplicated phase/status fields that the repo treats as wake-up-critical.
- Expected impact: reduces stateless-agent drift and makes future audits/execution sessions safer to resume.
- Risks / tradeoffs: low. The main tradeoff is choosing between stronger enforcement and less duplicated status text.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
  - a new or updated lane-state consistency check if one is added
- Product clarification required: No

### 3. [ ] Split the migration-facing native action dispatch hubs in `app_core` into smaller surface-specific helpers with direct local tests

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: `app_core` is the migration-facing dispatch boundary, but the heaviest browser and waveform action routing still lives in large branch-heavy hubs with mostly indirect coverage.
- Evidence:
  - `src/app_core/controller/browser_actions.rs` keeps `apply_browser_native_ui_action(...)` as a broad browser/source/folder/options/drag/cleanup matcher.
  - `src/app_core/controller/waveform_actions.rs` keeps `apply_waveform_native_ui_action(...)` as a broad waveform/edit/zoom/slice/destructive matcher.
  - `tmp/cleanup_audit_hotspots.md` reports `254` lines for `apply_browser_native_ui_action` and `279` lines for `apply_waveform_native_ui_action`.
  - Direct coverage today is concentrated in omnibus suites such as `src/app_core/controller/tests/dispatch/core.rs` rather than narrowly scoped tests beside the dispatch helpers.
- Recommended change: split browser/source/folder/options and waveform/edit/zoom/slice/destructive routes into focused helpers or submodules, then add local dispatch tests that characterize status/focus/error behavior without routing every assertion through the omnibus dispatch harness.
- Expected impact: lowers maintenance cost for future migration work and makes runtime-action regressions easier to localize.
- Risks / tradeoffs: medium. These tables are compatibility-sensitive and must preserve routing/status semantics exactly.
- Dependencies: none
- Suggested validation:
  - targeted `app_core` dispatch tests
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
- Product clarification required: No

### 4. [ ] Collapse duplicated keyboard-path logic in `vendor/radiant` so tests and production use the same hotkey/text-input behavior

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: the native runtime’s keyboard path is both over budget and partially duplicated across test-only helpers and the real event handler, which creates a drift risk in a focus-sensitive interaction surface.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_events/keyboard.rs` is `403` lines and currently fails the full file-size budget.
  - The file defines `handle_hotkey_press_for_tests`, `handle_character_key_for_tests`, `handle_enter_for_tests`, and `handle_escape_for_tests`.
  - The same enter/escape/text-target/hotkey branches also appear inside `handle_keyboard_input(...)`.
  - `docs/design_principles.md` treats contextual hotkeys and deterministic interaction as first-class behavior.
- Recommended change: extract shared helpers for escape/enter/text-input/hotkey routing and make both tests and the real event path exercise the same branch logic.
- Expected impact: reduces test-vs-runtime drift risk while shrinking one of the current non-allowlisted runtime files.
- Risks / tradeoffs: medium. Keyboard behavior is user-facing and regression-sensitive, so refactoring must preserve exact focus and prompt semantics.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test -p radiant` coverage for keyboard/runtime paths
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 5. [ ] Burn down the remaining non-allowlisted production/runtime file-size backlog before the oversized test hubs

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: L
- Why it matters: the full-scan file-size lane is still red, and the remaining production hotspots sit in active runtime/layout/browser surfaces where smaller modules would improve both guardrail health and day-to-day reasoning.
- Evidence:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` currently reports `20` non-allowlisted over-budget files.
  - Production/runtime hotspots inside that set include:
    - `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_toolbar.rs` (`421`)
    - `vendor/radiant/src/gui/layout_core/engine/context.rs` (`413`)
    - `vendor/radiant/src/gui_runtime/native_vello/input.rs` (`403`)
    - `vendor/radiant/src/gui_runtime/native_vello/runtime_events/keyboard.rs` (`403`)
    - `vendor/radiant/src/gui/native_shell/state/automation.rs` (`407`)
    - `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/worker.rs` (`406`)
    - `src/selection/range.rs` (`407`)
  - `docs/file_size_budget_allowlist.txt` already documents the intentionally centralized exceptions separately, so these non-allowlisted files are the sharper current debt.
- Recommended change: split this backlog in production-first order, starting with the vendor runtime/native-shell/layout files and the source-move worker; leave intentionally cohesive surfaces such as `src/selection/range.rs` for a later slice unless a clearer sub-boundary emerges.
- Expected impact: reduces the remaining truthful guardrail backlog and makes active runtime code easier to review and change safely.
- Risks / tradeoffs: medium. Behavior-preserving structural work in runtime/input/layout code is regression-sensitive.
- Dependencies: item 4 should be folded into this lane for `runtime_events/keyboard.rs`
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted unit/runtime tests for each split cluster
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 6. [ ] Split the oversized regression/test hubs so coverage stays discoverable and the file-size policy remains credible

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: L
- Why it matters: most of the remaining non-allowlisted backlog is now test code, and the largest suites bundle many behaviors together in ways that make failures, ownership, and future audit work harder to navigate.
- Evidence:
  - Oversized test hubs currently include:
    - `src/analysis/ann_index_tests.rs` (`422`)
    - `src/app/controller/playback/recording/waveform_loader/tests.rs` (`432`)
    - `src/app/controller/tests/browser_core/filters.rs` (`430`)
    - `src/app/controller/tests/browser_selection.rs` (`409`)
    - `src/app/controller/tests/waveform_cache_loading.rs` (`422`)
    - `src/sample_sources/db/file_ops_journal/tests.rs` (`423`)
    - `src/sample_sources/scanner/scan/tests.rs` (`407`)
    - `src/app_core/native_shell/tests/overlays.rs` (`409`)
    - `vendor/radiant/src/gui_runtime/native_vello/tests/browser_pointer/surface_routes.rs` (`412`)
    - `vendor/radiant/src/gui/native_shell/state/tests/browser_rows/virtualization.rs` (`406`)
    - `vendor/radiant/src/gui/native_shell/state/tests/overlays/waveform_hover.rs` (`403`)
  - These suites already contain meaningful behavior coverage; the current problem is size, discoverability, and maintenance friction rather than missing assertions.
- Recommended change: split the largest test hubs by sub-behavior or fixture family while preserving the current assertions and helper reuse.
- Expected impact: improves navigability of the remaining regression surface and supports the repo’s explicit small-file policy without sacrificing coverage.
- Risks / tradeoffs: medium. This is mostly mechanical, but careless moves can damage fixture readability or test helper reuse.
- Dependencies: none
- Suggested validation:
  - targeted test invocations for each split suite
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 7. [ ] Tighten `QUALITY_SCORE` drift enforcement or remove volatile exact counts from the scorecard narrative

- Classification: Documentation gap
- Confidence: High
- ROI: Medium
- Effort: S-M
- Why it matters: the score row is kept honest, but the surrounding narrative still cites stale exact counts, which weakens the value of the scorecard as a wake-up and prioritization artifact.
- Evidence:
  - `docs/QUALITY_SCORE.md` still says the live tree has `25` non-allowlisted over-budget Rust files and `2` files with `dead_code` suppressions.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` currently reports `20` non-allowlisted violations.
  - `tmp/cleanup_audit_hotspots.md` currently reports `27` total over-budget Rust files and `3` files with `dead_code` suppressions.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` still passes because it only checks the area score, not the narrative counts.
- Recommended change: either make the drift check validate the cited counts against the current audit output, or rewrite the narrative notes to avoid brittle exact numbers that are expected to change frequently.
- Expected impact: keeps the scorecard truthful without forcing humans to rediscover which parts of it are enforced versus advisory.
- Risks / tradeoffs: low. Stronger validation increases maintenance cost slightly; looser narrative wording reduces precision.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. Are playback-age filter inversion and temporary-mark review-advance semantics deliberate product contracts or only current behavior?

- Evidence:
  - `manual/usage.md` describes these flows using “Current behavior” wording.
  - `src/app/controller/library/wavs/browser_search/mutations.rs` implements additive/invert playback-age selection and marked-only browser behavior.
  - `src/app_core/actions/catalog/entries.rs` and `src/gui_test/aiv/packs/cases/browser.rs` expose these actions as stable runtime/test surfaces.
- Why this matters: safe future changes to browser filters, hotkeys, docs, and AIV assertions depend on knowing whether these semantics are intentional UX contracts or just today’s implementation.
- Affected files/modules:
  - `manual/usage.md`
  - `src/app/controller/library/wavs/browser_search/mutations.rs`
  - `src/app_core/actions/catalog/entries.rs`
  - `src/gui_test/aiv/packs/cases/browser.rs`
- Risk if guessed incorrectly: future work could accidentally lock in incidental behavior or regress a deliberate fast-triage workflow.
- Most conservative provisional assumption: preserve the current semantics exactly, but avoid broadening them into stronger product promises without explicit clarification.

### [!] 2. Should active-lane status have one canonical source, or is duplicated phase text across wake-up docs still intentional?

- Evidence:
  - `AGENTS.md`, `MEMORY.md`, `docs/plans/index.md`, and `docs/plans/active/todo.md` currently agree with each other.
  - `docs/README.md` currently disagrees and says the same lane is complete.
  - `AGENTS.md` explicitly treats these docs as stateless-agent wake-up artifacts.
- Why this matters: implementing item 2 safely requires knowing whether the repo wants duplicated mutable status text with stronger enforcement or a single canonical source with lightweight pointers.
- Affected files/modules:
  - `AGENTS.md`
  - `MEMORY.md`
  - `docs/README.md`
  - `docs/plans/index.md`
  - `docs/plans/active/todo.md`
- Risk if guessed incorrectly: the repo could either keep recurring status drift or add unnecessary process overhead to docs that were meant to stay lightweight.
- Most conservative provisional assumption: keep one short canonical lane-status source and make the other wake-up docs point to it rather than restating mutable phase details.

### [!] 3. Should stable action identifiers remain duplicated across the host catalog and vendor automation layer, or is a shared lower-layer contract intended?

- Evidence:
  - `docs/gui_test_platform.md` says the host action catalog in `src/app_core/actions` is canonical.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs` still keeps a manual `action_slug(...)` matcher.
  - `vendor/radiant/src/gui/native_shell/state/automation/*.rs` also hardcode several `available_actions` strings.
  - `src/gui_test/runner/tests.rs` now guards that advertised action ids resolve through the host catalog.
- Why this matters: safe cleanup of the remaining automation duplication depends on whether the boundary should stay split with tests as the safety net, or whether the repo wants a shared lower-layer action-id source.
- Affected files/modules:
  - `docs/gui_test_platform.md`
  - `src/app_core/actions/catalog/entries.rs`
  - `vendor/radiant/src/gui/native_shell/state/automation.rs`
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs`
- Risk if guessed incorrectly: a cleanup could either violate the intended `src`/`vendor` ownership boundary or preserve needless duplication longer than necessary.
- Most conservative provisional assumption: preserve the current boundary split and rely on stronger consistency tests unless the repo explicitly documents a shared lower-layer contract.

## Rejected Ideas

### [-] 1. Split the explicitly allowlisted centralized tables just to satisfy the file-size budget

- Why it was considered: `src/app_core/actions/catalog/kinds.rs`, `vendor/radiant/src/app/hotkeys.rs`, and `vendor/radiant/src/app/actions/mod.rs` are all large enough to attract cleanup attention.
- Why it was rejected: `docs/file_size_budget_allowlist.txt` explicitly justifies them as centralized compatibility surfaces, so raw size alone is not enough evidence to make them the next lane.
- What evidence was missing: concrete correctness, ownership, or discoverability failures caused by their current centralization.

### [-] 2. Treat `src/selection/range.rs` as an untested hotspot and move it to the top of the backlog

- Why it was considered: `tmp/cleanup_audit_hotspots.md` currently lists `src/selection/range.rs` under “Likely test-gap hotspots”.
- Why it was rejected: `src/selection/mod.rs` declares `mod tests;`, and `src/selection/tests.rs` already covers `SelectionRange` and related behavior. The audit heuristic is the real issue.
- What evidence was missing: proof that `SelectionRange` currently lacks targeted tests or that its current cohesion comment is false.

### [-] 3. Promote desktop AIV into CI or make it the next primary lane

- Why it was considered: the repo now has a broader `desktop-regression` pack and dedicated PowerShell wrappers.
- Why it was rejected: `docs/gui_test_platform.md` still describes desktop AIV as local-only because foreground activation remains unstable on the current Windows setup.
- What evidence was missing: repeatable local stability data and an explicit documented promotion bar for CI.
