# Improvement Audit Plan

Generated: 2026-03-31
Observed superproject commit: `a5393c80`
Observed `vendor/radiant` commit: `bb734080`
Observed workspace state: dirty worktree at audit start (root repo plus dirty `vendor/radiant`)
Status: Phase 2 completed on 2026-03-31; retained as the completed execution record for this audit lane.

## Scope

- This plan supersedes the previous execution record that lived at this path.
- Findings are ranked in strict execution order by expected ROI for the current live tree, not by category.
- Recommendations stay inside repository-supported direction. Broad rewrites, speculative features, and preference-only cleanup are excluded.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as an early-alpha Rust desktop tool for triaging, auditioning, editing, and curating local audio samples.
- Maturity level: Explicitly documented. `README.md` warns that the app is early alpha and can modify, rename, or delete sample-library files.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace with the root `sempal` crate, workspace apps/tools, and the vendored `radiant` GUI/runtime submodule.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` routes domain/controller work through `src/`, migration-facing projection/runtime glue through `src/app_core` and `src/gui_runtime`, GUI/runtime behavior through `vendor/radiant/`, and support tooling through `apps/` and `tools/`.
- Architectural boundaries: Explicitly documented. `docs/ARCHITECTURE.md` says domain state and UI intent belong in `src`, while `vendor/radiant` owns widget behavior, layout, hit testing, input routing, and rendering coordination.
- Test strategy: Strongly implied by code/docs. `docs/TEST.md` and `.github/workflows/ci.yml` center the repo on deterministic Rust unit tests, `cargo nextest`, targeted GUI contract tests, and optional desktop-AIV loops.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, non-blocking execution, predictability, reversibility, and data integrity.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, social network, or attention-retention product.

## Audit Baseline

- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` fails in mandatory preflight because `scripts/check_migration_boundary.ps1` finds direct `crate::app::` crossings under `src/app_core/**` outside `src/app_core/app_api.rs`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` reproduces the same failure directly. The live violations are in:
  - `src/app_core/controller/browser_actions.rs:192`
  - `src/app_core/controller/browser_actions.rs:195`
  - `src/app_core/controller/browser_actions.rs:198`
  - `src/app_core/native_shell.rs:169`
  - `src/app_core/native_shell/browser_projection/cache.rs:97`
  - `src/app_core/native_shell/browser_projection/cache.rs:172`
  - `src/app_core/native_shell/browser_projection/panel.rs:109`
  - `src/app_core/native_shell/browser_projection/panel.rs:112`
  - `src/app_core/native_shell/browser_projection/row_window.rs:82`
  - `src/app_core/native_shell/browser_projection/row_window.rs:195`
  - `src/app_core/native_shell/browser_projection/row_window.rs:198`
  - `src/app_core/native_shell/browser_projection/row_window.rs:201`
  - `src/app_core/native_shell/browser_projection/row_window.rs:204`
  - `src/app_core/native_shell/browser_projection/row_window.rs:207`
  - `src/app_core/native_shell/browser_projection/row_window.rs:222`
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` reports three active non-allowlisted file-budget violations:
  - `src/app_core/controller/tests/dispatch.rs` (`465` lines)
  - `vendor/radiant/src/gui/native_shell/state/tests/browser_rows/rendering.rs` (`447` lines)
  - `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_row_decor.rs` (`444` lines)
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` now reports the file-budget guardrail as failing and downgrades the live quality score lane to `3`.
- The durable agent/handoff docs are currently stale for this tree:
  - `AGENTS.md:64` still says Phase 2 completed and the lane is green again.
  - `MEMORY.md:24` and `MEMORY.md:25` still say the full file-size budget and `ci_agent` are green again.
  - `docs/plans/index.md:9` and `docs/plans/active/todo.md:14` still describe `tmp/improvement_audit_plan.md` as the completed execution record.
  - `docs/QUALITY_SCORE.md:30` and `docs/QUALITY_SCORE.md:38` still describe `30` active-scope file-budget violations, while the live full scan now reports `3`.
- User/developer docs do not currently explain the newly exposed browser playback-age and transient-mark workflows:
  - `manual/usage.md:47` still documents only `All/Keep/Trash/Untagged` browser chips.
  - `manual/usage.md:86` and `manual/usage.md:87` do not mention browser sample marks, marked-only filtering, or playback-age filtering.
  - `src/app_core/actions/catalog/entries.rs:180`, `src/app_core/actions/catalog/entries.rs:181`, and `src/app_core/actions/catalog/entries.rs:182` expose `toggle_browser_playback_age_filter`, `toggle_browser_sample_mark`, and `toggle_browser_marked_filter` as stable contract actions.
  - `src/gui_test/aiv/packs/cases/browser.rs:125` defines a live `browser_playback_age_filters` desktop-AIV case, but `docs/gui_test_platform.md:179` omits it from the documented `desktop-regression` pack list.

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for listening to, navigating, editing, and curating local sample libraries with strong emphasis on responsiveness and reversible workflows.
- What the repo appears to be moving toward: Strongly implied by code/docs. Tighter `app_core` migration boundaries, a truthful native-shell/action-catalog contract, browser filtering/marking affordances exposed through the native runtime, and continued enforcement of the file-size budget outside a small explicit allowlist.
- What is merely possible but unsupported: broad `app_core` redesigns, persistent browser-mark workflows, or broader GUI contract expansion beyond what current docs/tests and the action catalog already commit to.

## Ordered Backlog

### 1. [x] Restore the `app_core` migration boundary for browser playback-age dispatch and projection helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: S-M
- Why it matters: the mandatory preflight path is red on the live tree, and the current playback-age additions bypass the repo’s explicit rule that direct `crate::app::` crossings stay isolated to `src/app_core/app_api.rs`.
- Evidence:
  - `scripts/check_migration_boundary.ps1` fails on the live tree.
  - `src/app_core/controller/browser_actions.rs:192`, `src/app_core/controller/browser_actions.rs:195`, and `src/app_core/controller/browser_actions.rs:198` map native playback-age actions through `crate::app::state::PlaybackAgeFilterChip`.
  - `src/app_core/native_shell.rs:169` calls `crate::app::state::browser_playback_age_filter_chips()`.
  - `src/app_core/native_shell/browser_projection/cache.rs:97` and `src/app_core/native_shell/browser_projection/cache.rs:172` use `crate::app::state::PlaybackAgeBucket`.
  - `src/app_core/native_shell/browser_projection/panel.rs:109` and `src/app_core/native_shell/browser_projection/panel.rs:112` use `crate::app::state::PlaybackAgeFilterChip` and `crate::app::state::browser_playback_age_filter_chips()`.
  - `src/app_core/native_shell/browser_projection/row_window.rs:82`, `src/app_core/native_shell/browser_projection/row_window.rs:195`, and `src/app_core/native_shell/browser_projection/row_window.rs:198` still route bucket conversion through `crate::app::state::PlaybackAgeBucket`.
  - `src/app_core/state.rs:52` and `src/app_core/state.rs:55` already expose migration-facing aliases for `PlaybackAgeFilterChip` and `PlaybackAgeBucket`, which indicates the intended boundary surface already exists.
  - `docs/gui_migration_parity.md:161` still lists an older blocker set and does not describe the current browser-action/panel/cache violations now failing preflight.
- Recommended change: route the current playback-age browser action/projection callers back through `app_core::state` or `app_core::app_api::state`, add a migration-facing alias/helper for chip ordering if needed, and refresh `docs/gui_migration_parity.md` so the blocker list matches the live tree.
- Expected impact: restores the mandatory agent preflight gate, re-aligns the current playback-age slice with the documented migration boundary, and reduces future `app_core` drift.
- Risks / tradeoffs: low. The main risk is fixing import paths without clarifying longer-term type ownership, which could allow the same drift pattern to recur.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Execution notes:
  - Completed on `2026-03-31`.
  - Scope stayed at the path-level boundary repair: playback-age chip ordering now routes through `app_core::state`, and the remaining `app_core` callers no longer cross directly into `crate::app::state`.
  - Validation run:
    - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
    - `cargo test -p sempal --lib app_core::controller::tests::dispatch -- --test-threads=1`
  - Commit hash: recorded in the final Phase 2 report for this run.
  - Assumptions: deeper state-ownership migration remains out of scope.
  - Deviation from original order: none

### 2. [x] Restore the green full-scan file-size budget for the three current non-allowlisted hotspots

- Classification: Refactor / cleanup
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the live full-scan budget is red again, the quality-score drift lane now reports a degraded guardrail state, and the three current offenders are all actively touched files outside the explicit allowlist.
- Evidence:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` reports:
    - `src/app_core/controller/tests/dispatch.rs: 430`
    - `vendor/radiant/src/gui/native_shell/state/tests/browser_rows/rendering.rs: 418`
    - `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_row_decor.rs: 411`
  - Current line counts in the working tree are now `465`, `447`, and `444` respectively.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` now fails on the same file-budget guardrail.
  - `docs/file_size_budget_allowlist.txt` does not allowlist any of these three files.
  - `src/app_core/controller/tests/dispatch.rs` mixes grouped-dispatch coverage, playback-age filter coverage, marked-filter coverage, and sample-mark focus/loading assertions in one file.
  - `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_row_decor.rs` mixes locked/marked indicators, rating indicator layout, inline metadata chip layout, similarity button rendering, and row-border snapping in one production helper.
  - `vendor/radiant/src/gui/native_shell/state/tests/browser_rows/rendering.rs` mixes inline metadata, striping, locked/marked states, similarity highlighting, and playback-age rendering assertions in one test hub.
- Recommended change: split the three offenders along the boundaries already visible in the code:
  - `dispatch.rs`: separate browser/search/mark dispatch assertions from waveform/update/general routing checks.
  - `browser_row_decor.rs`: separate rating indicators, inline metadata chips, and similarity/button-border helpers.
  - `rendering.rs`: split browser-row rendering tests into metadata/selection/state-specific modules.
- Expected impact: restores the full-scan file-size budget to green, keeps the quality-score lane honest, and makes future failures easier to localize.
- Risks / tradeoffs: medium. The work is behavior-preserving structural churn, so the main risk is making test selection or vendor module ownership harder to follow if the split boundaries are weak.
- Dependencies: item 1 should land first if the same playback-age slice is still being edited.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - targeted `cargo test` filters for split root-side and `vendor/radiant` modules
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Execution notes:
  - Completed on `2026-03-31`.
  - `src/app_core/controller/tests/dispatch.rs`, `vendor/radiant/src/gui/native_shell/state/tests/browser_rows/rendering.rs`, and `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_row_decor.rs` were split into focused submodules.
  - Validation uncovered one safe prerequisite fix in `vendor/radiant/src/gui/native_shell/state/tests/mod.rs`, where the shared `CachedBrowserRow` test helper needed the new `playback_age_bucket` field initialized.
  - Validation run:
    - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
    - `cargo test -p sempal --lib app_core::controller::tests::dispatch -- --test-threads=1`
    - `cargo test -p radiant --lib browser_rows -- --test-threads=1`
  - Commit hash: recorded in the final Phase 2 report for this run.
  - Assumptions: the prerequisite `CachedBrowserRow` field fix was behavior-preserving test maintenance, not a product change.
  - Deviation from original order: none

### 3. [x] Reconcile the broader agent-facing status docs and quality score after the live blockers are fixed

- Classification: Developer-experience improvement
- Confidence: High
- ROI: Medium-High
- Effort: S
- Why it matters: Sempal explicitly relies on stateless wake-up files and scorecards for orientation, but the current durable docs disagree with the live tree about both the active lane and the current guardrail state.
- Evidence:
  - `AGENTS.md:64` still says Phase 2 completed and `scripts/check_file_size_budget.ps1 -All`, `scripts/devcheck.ps1`, and `scripts/ci_agent.ps1` are green again.
  - `MEMORY.md:24`, `MEMORY.md:25`, and `MEMORY.md:46` still describe the previous execution lane as complete and green.
  - `docs/plans/index.md:9` and `docs/plans/active/todo.md:14` still describe `tmp/improvement_audit_plan.md` as the completed execution record.
  - `docs/QUALITY_SCORE.md:7`, `docs/QUALITY_SCORE.md:30`, and `docs/QUALITY_SCORE.md:38` still describe the last reviewed tree as having `30` active-scope file-budget violations.
  - The live checks now disagree:
    - `scripts/run_agent_request.ps1` fails in preflight.
    - `scripts/check_quality_score_drift.ps1` reports a downgraded live guardrail state.
    - `scripts/check_file_size_budget.ps1 -All` reports `3` active violations, not `30`.
- Recommended change: after items 1 and 2 land, refresh `docs/QUALITY_SCORE.md` and the agent-facing wake-up files so they reflect the then-current guardrail status, counts, dates, and active lane truthfully.
- Expected impact: future agent sessions start from an accurate status picture instead of stale “green again” claims or stale violation counts.
- Risks / tradeoffs: low. The only real risk is updating the scorecard/status text before rerunning the actual guardrails.
- Dependencies: items 1 and 2, or at minimum a fresh rerun of the relevant guardrails before any “green” claim is written.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
- Product clarification required: No
- Execution notes:
  - Completed on `2026-03-31`.
  - `AGENTS.md`, `MEMORY.md`, `docs/README.md`, `docs/plans/index.md`, `docs/plans/active/todo.md`, and `docs/QUALITY_SCORE.md` were rewritten to match the live repaired tree instead of the stale degraded snapshot.
  - Validation run:
    - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
    - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
    - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
  - Commit hash: recorded in the final Phase 2 report for this run.
  - Assumptions: only rerun checks are described as green.
  - Deviation from original order: none

### 4. [x] Document the browser playback-age and transient-mark workflows across user and developer docs

- Classification: Documentation gap
- Confidence: High
- ROI: Medium
- Effort: S-M
- Why it matters: the native runtime, action catalog, and desktop-AIV pack already expose playback-age filters, transient sample marks, and marked-only filtering as supported surfaces, but the user-facing docs still describe an older browser workflow.
- Evidence:
  - `manual/usage.md:47` still says the browser filter chips are `All/Keep/Trash/Untagged`.
  - `manual/usage.md:86` and `manual/usage.md:87` do not mention playback-age filters, sample marks, or a marked-only filter flow.
  - `src/app_core/actions/catalog/entries.rs:180`, `src/app_core/actions/catalog/entries.rs:181`, and `src/app_core/actions/catalog/entries.rs:182` treat `toggle_browser_playback_age_filter`, `toggle_browser_sample_mark`, and `toggle_browser_marked_filter` as stable contract actions.
  - `src/gui_test/aiv/packs.rs:54` includes the browser playback-age filters case in the `desktop-regression` pack.
  - `src/gui_test/aiv/packs/cases/browser.rs:125` defines the `browser_playback_age_filters` desktop-AIV case.
  - `docs/gui_test_platform.md:179` does not list `browser_playback_age_filters` in the documented `desktop-regression` pack.
  - Repository-wide doc search found no user/developer docs describing the playback-age chips or transient browser sample marks outside code comments and tests.
- Recommended change: update `manual/usage.md` and relevant developer docs to explain the current browser affordances, their UI labels, and how they relate to existing search/filter flows; also update `docs/gui_test_platform.md` so the documented desktop-AIV pack list matches the current manifest.
- Expected impact: improves feature discoverability, makes the GUI contract more truthful, and reduces the risk of future changes guessing at undocumented browser semantics.
- Risks / tradeoffs: medium. This item should not overstate semantics that are still ambiguous; see the open questions below before turning current behavior into a hard user-facing contract.
- Dependencies: resolve or explicitly bracket the open questions below when the docs would otherwise turn current behavior into a normative promise.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
- Product clarification required: Yes
- Execution notes:
  - Completed on `2026-03-31`.
  - `manual/usage.md` now documents the current browser playback-age chips (`NVR`, `1M`, `1W`), the `MARK` filter, the `;` sample-mark hotkey, and the current single-row follow-up behavior using explicit “current behavior” wording.
  - `docs/gui_test_platform.md` now lists the live `browser_playback_age_filters` desktop-AIV regression case in the documented `desktop-regression` pack.
  - Validation run:
    - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
    - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
    - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - Commit hash: recorded in the final Phase 2 report for this run.
  - Assumptions: unresolved product intent stays bracketed as current behavior, not a stronger long-term promise.
  - Deviation from original order: none

## Final Validation Summary

- `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`

## Open Questions / Missing Definitions

### [!] 1. Is `app_core` still expected to narrow legacy state ownership beyond alias-level centralization?

- Evidence:
  - `docs/gui_migration_parity.md:82` and `docs/gui_migration_parity.md:86` describe `app_core::app_api` as the central location for direct legacy crossings.
  - `src/app_core/state.rs:52` and `src/app_core/state.rs:55` currently provide alias-level playback-age types but do not yet own independent semantics.
  - The live violations in `src/app_core/controller/browser_actions.rs` and `src/app_core/native_shell/browser_projection/**` show that path-level centralization is still brittle.
- Why this matters: item 1 has a clear minimal fix, but future work could overreach into a broader state-ownership refactor without stronger repository evidence.
- Affected files/modules:
  - `src/app_core/app_api.rs`
  - `src/app_core/state.rs`
  - `src/app_core/controller/browser_actions.rs`
  - `src/app_core/native_shell/**`
  - `docs/gui_migration_parity.md`
- Risk if guessed incorrectly: a small alias repair could be mistaken for “migration complete,” or a larger refactor could widen scope beyond what the repo currently justifies.
- Most conservative provisional assumption: repair the import-path boundary only and treat deeper ownership narrowing as separate future work unless new docs or explicit user direction say otherwise.

### [!] 2. Are browser playback-age filters intentionally additive, and is the “invert into all other chips” behavior part of the user contract?

- Evidence:
  - `src/app/controller/library/wavs/browser_search/mutations.rs:62` adds/removes playback-age chips when `additive` is true.
  - `src/app/controller/library/wavs/browser_search/mutations.rs:138` and `src/app/controller/library/wavs/browser_search/mutations.rs:160` define and apply the invert-other-chips behavior.
  - `src/app_core/controller/tests/dispatch.rs:360` asserts the invert-and-reclick-clear behavior.
  - `manual/usage.md` does not describe these filters at all.
- Why this matters: documenting or extending this workflow safely requires knowing whether the current additive/invert behavior is intentional UX or merely current implementation.
- Affected files/modules:
  - `src/app/controller/library/wavs/browser_search/mutations.rs`
  - `src/app/state/browser/search.rs`
  - `src/app_core/controller/tests/dispatch.rs`
  - `manual/usage.md`
- Risk if guessed incorrectly: docs, tests, and future UI polish could lock in behavior that the product does not actually intend to preserve.
- Most conservative provisional assumption: keep the current additive-set and invert-other-chips behavior unchanged, but document it as current behavior rather than as a deeply committed long-term UX unless clarified.

### [!] 3. Are transient browser sample marks intentionally session-scoped and supposed to auto-advance focus/load the next sample?

- Evidence:
  - `src/app/state/browser.rs:33` describes sample marks as session-scoped.
  - `src/app/controller/library/wavs/browser_search/mutations.rs:28` and `src/app/controller/library/wavs/browser_pipeline/helpers.rs:80` treat `marked_only` as a live browser filter dimension.
  - `src/app_core/controller/tests/dispatch.rs:406` asserts that `ToggleBrowserMarkedFilter` enables `marked_only`.
  - `src/app_core/controller/tests/dispatch.rs:453`, `src/app_core/controller/tests/dispatch.rs:457`, and `src/app_core/controller/tests/dispatch.rs:462` assert that `ToggleBrowserSampleMark` both marks the focused sample and advances focus/loading to `next.wav`.
  - `manual/usage.md` does not describe the mark workflow, its lifetime, or its navigation effects.
- Why this matters: user-facing docs and future browser-automation changes need a stable answer on whether marks are a transient working set, a future persistent concept, or something in between.
- Affected files/modules:
  - `src/app/state/browser.rs`
  - `src/app/controller/library/wavs/browser_marks.rs`
  - `src/app_core/controller/tests/dispatch.rs`
  - `manual/usage.md`
- Risk if guessed incorrectly: future changes could either remove a deliberate fast-triage flow or accidentally formalize a transient implementation detail into a product promise.
- Most conservative provisional assumption: preserve marks as session-only and keep the current focus-advance behavior unless explicit product direction says otherwise.

## Rejected Ideas

### [-] 1. Resume the broader historical cleanup backlog as the next lane

- Why it was considered: `tmp/cleanup_plan.md` and `tmp/cleanup_audit_hotspots.md` still describe a wider long-tail cleanup program.
- Why it was rejected: the live full-scan file-budget guardrail is currently red for only three active files, and the user asked for a fresh current-tree audit, not a reopening of the parked cleanup lane.
- What evidence was missing: proof that the older broader cleanup snapshot is the highest-ROI next action for the current live tree.

### [-] 2. Treat the current playback-age and transient-mark additions as justification for a broader browser-workflow redesign

- Why it was considered: the current browser feature slice touches filters, marks, automation, AIV packs, and migration-boundary code.
- Why it was rejected: the concrete live issues are a broken `app_core` boundary, a small set of file-budget regressions, and missing docs/contracts. A broader redesign is not justified by the current repository evidence.
- What evidence was missing: repository-specific proof of a broader architecture mismatch beyond the current bounded defects and doc gaps.

### [-] 3. Reintroduce broad tool/CLI parser work as part of this lane

- Why it was considered: `tools/gui-test-cli` remains a manual parser and is in the dirty worktree.
- Why it was rejected: the current live diff in `tools/gui-test-cli/src/main.rs` is formatting-only, existing tests already cover command parsing, and the repo’s stronger current evidence points elsewhere.
- What evidence was missing: a concrete parser correctness bug or maintenance failure caused by the current CLI shape.
