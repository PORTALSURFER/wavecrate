# Improvement Audit Plan

Generated: 2026-03-31
Observed superproject commit: `ce005592`
Observed `vendor/radiant` commit: `6f4b87d2`
Observed workspace state at audit start: clean worktree
Status: Phase 2 implementation is in progress on the approved backlog.

## Scope

- This plan supersedes the previous completed execution record that lived at this path.
- Findings are ranked in strict execution order by expected ROI for the current live tree.
- Recommendations stay inside repository-supported direction. Broad rewrites, speculative features, and preference-only cleanup are excluded.
- No implementation is included in this phase.

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

- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1 -SkipCi` is green on the current tree, so the repo is not currently blocked by the mandatory preflight lane.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` regenerated `tmp/cleanup_audit_hotspots.md` and reports `32` Rust files over the 400-line budget on a full physical-line scan.
- The Windows file-size budget guardrail does not currently agree with that hotspot snapshot:
  - `scripts/check_file_size_budget.ps1:159` measures line count with `Get-Content -LiteralPath $file | Measure-Object -Line`, which ignores blank lines.
  - `scripts/audit_cleanup_hotspots.ps1` uses `[System.IO.File]::ReadAllLines(...).Count`, and the bash side uses `wc -l`, so the repo currently has two definitions of “line count”.
  - Direct comparisons on the live tree show the mismatch clearly:
    - `src/app/controller/tests/browser_core/filters.rs`: `381` in the enforced PowerShell check vs `430` physical lines.
    - `src/app/controller/history.rs`: `398` vs `424`.
    - `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_toolbar.rs`: `395` vs `421`.
- Using physical-line counts and excluding the explicit allowlist in `docs/file_size_budget_allowlist.txt`, the live tree still has `25` non-allowlisted over-budget Rust files. The highest-value production hotspots are:
  - `src/app/controller/playback/transport/seek.rs` (`432`)
  - `src/app/controller/history.rs` (`424`)
  - `src/app/controller/library/source_folders/delete_recovery/recovery.rs` (`419`)
  - `src/app/controller/playback/player/playback_start.rs` (`404`)
  - `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_toolbar.rs` (`421`)
  - `vendor/radiant/src/gui/layout_core/engine/context.rs` (`413`)
  - `vendor/radiant/src/gui_runtime/native_vello/input.rs` (`403`)
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_events/keyboard.rs` (`403`)
- The loaded-sample similarity sort currently has duplicated query-building logic:
  - `src/app/controller/library/wavs/similar/query.rs:31` `build_similarity_query_for_loaded_sample`
  - `src/app/controller/library/wavs/similar/background.rs:198` `compute_loaded_similarity_query`
  - Both paths load the same embedding/features rows, compute the same embed/DSP blend, backfill missing rows with the same sentinel, and finish with `ensure_anchor_similarity_result(...)`.
- The GUI automation/action-id contract is still duplicated across the host catalog and native-shell automation surfaces:
  - `src/app_core/actions/catalog/entries.rs` is the canonical action-id catalog.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs:98` keeps a second manual `action_slug(...)` matcher.
  - `vendor/radiant/src/gui/native_shell/state/automation/browser.rs`, `dialogs.rs`, `sidebar.rs`, and `waveform.rs` also hardcode `available_actions` strings directly.
  - `src/gui_test/runner/assertions.rs:86` can validate one action id against the host catalog, but no blanket test traverses a real runtime automation snapshot and proves every advertised action resolves through `action_catalog_entry_by_id(...)`.
- Contextual hotkeys are a first-class documented interaction contract, but their direct local characterization remains thin relative to the surface area:
  - `docs/design_principles.md` calls contextual hotkeys and focus-sensitive keyboard semantics first-class.
  - `vendor/radiant/src/app/hotkeys.rs` is `954` physical lines and defines `82` `HotkeyBinding` entries plus the shared `resolve_hotkey_press(...)` resolver.
  - The same file currently contains only `6` local test functions.
- The migration-facing native dispatch layer still concentrates broad UI behavior in two large match tables:
  - `src/app_core/controller/browser_actions.rs:12` `apply_browser_native_ui_action(...)` spans the browser/source/folder action surface in one function.
  - `src/app_core/controller/waveform_actions.rs:12` `apply_waveform_native_ui_action(...)` spans waveform, edit, drag, zoom, and destructive actions in one function.
  - Coverage is currently indirect through `src/app_core/controller/tests/dispatch/**` rather than local dispatch-focused tests beside those modules.

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for listening to, navigating, editing, and curating local sample libraries with strong emphasis on responsiveness and reversible workflows.
- What the repo appears to be moving toward: Strongly implied by code/docs. More truthful guardrails, a stable `app_core`/`radiant` contract surface, deterministic GUI automation, and continued file-size/ownership discipline without reopening speculative runtime rewrites.
- What is merely possible but unsupported: broad GUI/runtime redesigns, a forced split of every centralized contract table, or promotion of desktop AIV into CI without stronger stability evidence.

## Ordered Backlog

### 1. [x] Fix the Windows file-size budget check so the guardrail counts physical lines instead of non-empty lines

- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the repository’s Windows-first guardrail lane can currently report a green 400-line budget while the live tree still violates the documented limit. That makes the audit, quality-score drift check, and local preflight disagree about the actual state of the codebase.
- Evidence:
  - `scripts/check_file_size_budget.ps1:159` uses `Get-Content -LiteralPath $file | Measure-Object -Line`.
  - `scripts/audit_cleanup_hotspots.ps1` uses `[System.IO.File]::ReadAllLines(...).Count`.
  - Direct comparisons on the live tree:
    - `src/app/controller/tests/browser_core/filters.rs`: `381` vs `430`.
    - `src/app/controller/history.rs`: `398` vs `424`.
    - `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_toolbar.rs`: `395` vs `421`.
  - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1 -SkipCi` is green while `tmp/cleanup_audit_hotspots.md` reports `32` over-budget files.
- Recommended change: make the PowerShell budget check use the same physical-line counting rule as the hotspot audit and bash lane, then add a script-guardrail fixture that includes blank lines so this regression cannot silently return.
- Expected impact: restores trust in the Windows guardrail lane, makes `check_quality_score_drift.ps1` meaningful again, and exposes the real backlog instead of hiding it behind a counting bug.
- Risks / tradeoffs: low. The likely side effect is a temporary red lane because the current tree will be measured truthfully.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1 -SkipCi`
  - script-guardrail fixture coverage for blank-line counting
- Product clarification required: No
- Execution date: `2026-03-31`
- Commit hash: `a5faedbe` (`fix: align file-size guardrails with physical line counts`)
- Assumptions used:
  - `docs/QUALITY_SCORE.md` should describe the current observed guardrail state, even when the truthful full-scan budget is red.
  - `check_quality_score_drift.*` is intended to validate the full-scan budget state because its score note already references `check_file_size_budget.* --all`.
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1` passed, including the new blank-line-count regression fixture.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` now fails truthfully and reports `25` non-allowlisted over-budget Rust files on the live tree.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` passed and now reports a downgraded guardrail score of `3` while the truthful full-scan budget remains red.
  - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1 -SkipCi` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed.
- Plan-order deviation: none

### 2. [x] Deduplicate loaded-sample similarity query construction so sync and background similarity sort share one implementation

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: loaded-sample similarity sort is a user-visible ranking feature, and today the same SQL, score blending, missing-entry fallback, and anchor-restoration logic exist in two places. That is a real drift risk: a future change can silently make “sort by loaded sample” behave differently depending on whether it runs synchronously or through the follow-loaded background refresh path.
- Evidence:
  - `src/app/controller/library/wavs/similar/query.rs:31` `build_similarity_query_for_loaded_sample(...)`
  - `src/app/controller/library/wavs/similar/background.rs:198` `compute_loaded_similarity_query(...)`
  - Both implementations:
    - load the current sample embedding and optional DSP features
    - query `embeddings` + `features`
    - compute `EMBED_WEIGHT * embed_sim + DSP_WEIGHT * dsp_sim`
    - append `MISSING_SIMILARITY_SCORE` for entries without embeddings
    - finish through `ensure_anchor_similarity_result(...)`
- Recommended change: extract a shared pure helper for loaded-sample similarity query construction and reuse it from both codepaths, leaving only the controller/job wiring separate. Add one parity test that proves both entrypoints produce the same `SimilarQuery` for the same source snapshot.
- Expected impact: removes a correctness drift risk from a visible ranking feature and lowers the maintenance cost of future similarity-policy changes.
- Risks / tradeoffs: medium. Shared extraction must preserve current sort order, sentinel behavior, and anchor placement exactly.
- Dependencies: none
- Suggested validation:
  - targeted controller similarity tests for both loaded-sort entrypoints
  - regression coverage around `ensure_anchor_similarity_result(...)`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Execution date: `2026-03-31`
- Commit hash: `pending until the item-2 commit is created`
- Assumptions used:
  - `build_similarity_query_for_loaded_sample(...)` remains a valid sync entrypoint even though current call sites are concentrated on the background follow-loaded path.
  - Preserving the exact loaded-query payload (`sample_id`, label, indices, scores, anchor placement, and missing-entry fallback ordering) is more important than reducing every small setup duplication around job/controller input acquisition.
- Validation outcome:
  - `cargo test -p sempal loaded_similarity -- --test-threads=1` passed, including a new parity test that seeds one source snapshot and compares the sync and background loaded-query builders.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed.
- Plan-order deviation: none

### 3. [ ] Add a repo-wide automation-snapshot to action-catalog consistency guard for advertised action ids

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: S
- Why it matters: the GUI automation tree is now a contract surface for the CLI runner, scenario packs, and desktop AIV. Today the host catalog and native-shell automation snapshot still carry duplicated action-id strings without one blanket test proving they stay aligned.
- Evidence:
  - `src/app_core/actions/catalog/entries.rs` is the canonical host action-id catalog.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs:98` keeps a second manual `action_slug(...)` matcher.
  - `vendor/radiant/src/gui/native_shell/state/automation/browser.rs`, `dialogs.rs`, `sidebar.rs`, and `waveform.rs` hardcode `available_actions` strings directly.
  - `src/gui_test/runner/assertions.rs:86` can validate one action id against the catalog, but `src/gui_test/runner/tests.rs` only exercises synthetic snapshots and vendor automation tests cover only selected nodes/strings.
- Recommended change: add one real-snapshot consistency test that traverses every `available_actions` id emitted by the native shell and proves each resolves through `action_catalog_entry_by_id(...)`; if practical, reduce string duplication by routing more action-id generation through one shared source.
- Expected impact: turns a manual contract surface into an enforced one and makes future GUI-action additions safer for the CLI, semantic runner, and desktop-AIV packs.
- Risks / tradeoffs: low. The main risk is exposing existing mismatches that require a small follow-up cleanup.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 4. [ ] Expand direct hotkey contract characterization around focus scope, chord routing, and collision-prone bindings

- Classification: Test gap
- Confidence: High
- ROI: Medium
- Effort: S-M
- Why it matters: Sempal’s design principles treat contextual hotkeys as part of the core interaction model, but the direct hotkey tests still cover only a small sample of a large shared binding surface. That makes future keyboard changes easier to regress silently.
- Evidence:
  - `docs/design_principles.md` defines hotkeys as contextual, focus-dependent, and central to the UI model.
  - `vendor/radiant/src/app/hotkeys.rs` is `954` physical lines and defines `82` `HotkeyBinding` entries plus `resolve_hotkey_press(...)`.
  - `docs/file_size_budget_allowlist.txt` explicitly keeps this file centralized for now, which makes direct characterization more important.
  - The same file currently has only `6` local test functions.
- Recommended change: keep the central table for now, but add focused tests for scope-sensitive gesture reuse (`C`, `D`, `N`, `R`, arrow keys), chord handling, and representative browser/folder/waveform conflicts so future additions fail loudly when they violate the intended focus contract.
- Expected impact: increases confidence in a first-class interaction surface without forcing a premature structural rewrite of an intentionally centralized table.
- Risks / tradeoffs: low. More tests will slow hotkey churn slightly, but they match the repo’s stated interaction priorities.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml hotkeys -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 5. [ ] Burn down the highest-ROI non-allowlisted full-scan file-size backlog once the measurement bug is fixed

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: L
- Why it matters: the repo’s 400-line policy is real, but the current Windows guardrail bug is hiding a production/test backlog of `25` non-allowlisted files. Leaving that debt hidden encourages more drift in areas the project explicitly wants to keep small and focused.
- Evidence:
  - Physical-line full scan on the current tree shows `25` non-allowlisted over-budget Rust files.
  - Highest-value production hotspots:
    - `src/app/controller/playback/transport/seek.rs` (`432`)
    - `src/app/controller/history.rs` (`424`)
    - `src/app/controller/library/source_folders/delete_recovery/recovery.rs` (`419`)
    - `src/app/controller/playback/player/playback_start.rs` (`404`)
    - `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_toolbar.rs` (`421`)
    - `vendor/radiant/src/gui/layout_core/engine/context.rs` (`413`)
    - `vendor/radiant/src/gui_runtime/native_vello/input.rs` (`403`)
    - `vendor/radiant/src/gui_runtime/native_vello/runtime_events/keyboard.rs` (`403`)
  - The repo explicitly treats oversized allowlisted stability surfaces separately in `docs/file_size_budget_allowlist.txt`, so these non-allowlisted files are the sharper current debt.
- Recommended change: after item 1 restores truthful measurement, split the non-allowlisted backlog in ROI order, starting with production hotspots before test hubs. Preserve the explicit allowlist distinction instead of reopening intentionally centralized contract tables.
- Expected impact: restores the credibility of the 400-line policy on the live tree and reduces future merge/debug friction in active controller/runtime areas.
- Risks / tradeoffs: medium. This is behavior-preserving structural work with regression risk if boundaries are chosen weakly.
- Dependencies: item 1
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted unit/integration tests for each split cluster
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 6. [ ] Split the migration-facing native action dispatch hubs in `app_core` into smaller surface-specific helpers with direct local tests

- Classification: Architecture improvement
- Confidence: Medium-High
- ROI: Medium
- Effort: M
- Why it matters: `app_core` is the migration-facing native dispatch boundary, but two large match tables still own most browser/source/folder and waveform/edit/zoom behavior. That concentration makes boundary work and UI-action changes harder to reason about than the surrounding module decomposition suggests.
- Evidence:
  - `src/app_core/controller/browser_actions.rs` is `320` lines and keeps `apply_browser_native_ui_action(...)` as one broad match over browser, source, folder, options, drag, and duplicate-cleanup actions.
  - `src/app_core/controller/waveform_actions.rs` is `360` lines and keeps `apply_waveform_native_ui_action(...)` as one broad match over waveform playback, selection, drag, edit, slice, zoom, and destructive actions.
  - Coverage is indirect through `src/app_core/controller/tests/dispatch/**` rather than direct tests beside the dispatch modules themselves.
  - The repo guidance in `AGENTS.md` asks for small focused modules and short functions, while these two migration-facing entrypoints remain branch-heavy hubs.
- Recommended change: split the browser/source/folder/options and waveform/edit/zoom/slice/destructive branches into focused helpers or submodules, then add local dispatch tests that characterize status/error/focus behavior without routing every assertion through the larger omnibus dispatch suite.
- Expected impact: lowers the maintenance cost of future native-action work in `app_core` and makes migration-boundary changes easier to localize and validate.
- Risks / tradeoffs: medium. The action tables are compatibility-sensitive, so refactors must preserve exact routing and status semantics.
- Dependencies: none
- Suggested validation:
  - targeted `app_core` dispatch tests
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. Which oversized centralized tables are intentionally exempt from the repo’s “small file” preference, and which are still fair cleanup targets?

- Evidence:
  - `docs/file_size_budget_allowlist.txt` explicitly preserves `src/app_core/actions/catalog/kinds.rs`, `vendor/radiant/src/app/hotkeys.rs`, and `vendor/radiant/src/app/actions/mod.rs` as centralized compatibility surfaces for now.
  - The same repo guidance also states a strong 400-line preference and small-focused modules in `AGENTS.md`.
- Why this matters: the next cleanup lane should not waste effort splitting files that the repo has consciously centralized unless there is clear correctness or ownership pressure beyond raw size.
- Affected files/modules:
  - `docs/file_size_budget_allowlist.txt`
  - `src/app_core/actions/catalog/kinds.rs`
  - `vendor/radiant/src/app/hotkeys.rs`
  - `vendor/radiant/src/app/actions/mod.rs`
- Risk if guessed incorrectly: either the audit overreaches into intentional compatibility surfaces, or it keeps avoidable debt indefinitely by treating all large central tables as untouchable.
- Most conservative provisional assumption: keep explicitly allowlisted compatibility surfaces centralized until a concrete correctness, ownership, or discoverability problem justifies a different shape.

### [!] 2. Are the browser playback-age and temporary mark behaviors now long-term product contracts, or are they still “current behavior” only?

- Evidence:
  - `manual/usage.md` documents playback-age and mark flows using “Current behavior” wording.
  - `src/app/controller/library/wavs/browser_search/mutations.rs` still contains additive/invert playback-age behavior and marked-only navigation side effects.
  - `src/app_core/actions/catalog/entries.rs` and the GUI automation/AIV packs expose those actions as stable runtime surfaces.
- Why this matters: future docs, hotkey changes, and GUI assertions should know whether they are preserving a deliberate UX contract or merely today’s implementation.
- Affected files/modules:
  - `manual/usage.md`
  - `src/app/controller/library/wavs/browser_search/mutations.rs`
  - `src/app_core/actions/catalog/entries.rs`
  - `src/gui_test/aiv/packs/cases/browser.rs`
- Risk if guessed incorrectly: either future work overcommits to incidental behavior or accidentally regresses a deliberate fast-triage workflow.
- Most conservative provisional assumption: preserve current playback-age and mark behavior, but avoid turning it into a broader product promise without explicit clarification.

### [!] 3. Should desktop AIV remain a local-only evidence lane, or is there a documented path to promoting part of it into CI?

- Evidence:
  - `docs/gui_test_platform.md` explicitly calls desktop AIV local-only and says it is not yet stable enough for CI.
  - The same doc also keeps `desktop-regression` as an important broader evidence lane.
- Why this matters: some future testing investments make sense only if there is intent to promote a stable subset, while others should stay focused on local triage ergonomics.
- Affected files/modules:
  - `docs/gui_test_platform.md`
  - `scripts/run_gui_aiv_smoke.ps1`
  - `scripts/run_gui_aiv_suite.ps1`
  - `src/gui_test/aiv/**`
- Risk if guessed incorrectly: the project could spend effort engineering CI around a lane that intentionally remains local-only, or miss the chance to harden a subset that is intended for eventual promotion.
- Most conservative provisional assumption: keep desktop AIV local-only and avoid CI-promotion work unless the docs or user direction become more explicit.

## Rejected Ideas

### [-] 1. Split explicitly allowlisted stability surfaces just to satisfy the file-size target

- Why it was considered: `src/app_core/actions/catalog/kinds.rs`, `vendor/radiant/src/app/hotkeys.rs`, and `vendor/radiant/src/app/actions/mod.rs` are all large enough to attract cleanup attention.
- Why it was rejected: `docs/file_size_budget_allowlist.txt` explicitly justifies those files as centralized compatibility surfaces, so raw size alone is not enough evidence to make them the top lane.
- What evidence was missing: concrete correctness, ownership, or discoverability failures caused by their current centralization.

### [-] 2. Reopen the parked perf lane or promote desktop AIV into CI as the next lane

- Why it was considered: the repo has a completed perf execution record and a broader desktop-AIV regression pack.
- Why it was rejected: `tmp/perf_plan.md` is parked/completed, `docs/gui_test_platform.md` still marks desktop AIV local-only, and the current stronger evidence is around guardrail truthfulness and contract drift.
- What evidence was missing: current-tree proof that performance or CI-promotion work is a higher-ROI need than the guardrail/contract issues above.

### [-] 3. Make the native-shell overlay test hub the first cleanup slice

- Why it was considered: `src/app_core/native_shell/tests/overlays.rs` is `409` physical lines and sits in a migration-sensitive GUI contract area.
- Why it was rejected: it is a reasonable cleanup candidate, but the current repo has sharper cross-cutting issues first: a false-green Windows guardrail, duplicated loaded-similarity logic, and missing cross-surface contract guards.
- What evidence was missing: proof that the overlay test hub is causing a more immediate correctness or workflow failure than the higher-ranked items.
