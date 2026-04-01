# Improvement Audit Plan

Generated: 2026-04-01
Observed superproject commit: `b79a2869`
Observed `vendor/radiant` commit: `d4dac5da`
Observed workspace state at audit start: dirty worktree (`MEMORY.md`)
Status: Phase 2 active on `2026-04-01`; item 1 is complete, item 2 is next, and execution is proceeding in backlog order.

## Scope

- This plan audits the current live tree only.
- Findings are ranked in strict execution order by expected ROI for the current repository state.
- Recommendations stay inside documented or strongly implied repository intent.
- No implementation was performed during this audit.
- Broad rewrites, speculative features, and preference-only cleanup are excluded.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as an early-alpha Rust desktop tool for triaging, auditioning, editing, and curating local audio samples with a strong listening-first workflow.
- Maturity level: Explicitly documented. `README.md` warns that the app is early alpha and can modify, rename, or delete library files.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace with the root `sempal` crate, companion apps/tools, and the vendored `radiant` GUI/runtime submodule.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` routes domain/controller logic through `src/`, migration-facing projection/runtime glue through `src/app_core` and `src/gui_runtime`, GUI/runtime behavior through `vendor/radiant/`, and support tooling through `apps/` and `tools/`.
- Architectural boundaries: Explicitly documented. `docs/ARCHITECTURE.md` says domain state and UI intent belong in `src`, while `vendor/radiant` owns widget behavior, layout, hit testing, input routing, and rendering coordination.
- Test strategy: Strongly implied by code/docs. `docs/TEST.md` and `.github/workflows/ci.yml` center the repo on deterministic Rust unit tests, `cargo nextest`, focused GUI contract tests, and optional local-only desktop-AIV loops.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, `scripts/ci_local.ps1`, and GUI-specific wrappers such as `scripts/run_gui_contract.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, non-blocking execution, deterministic input, reversibility, and data integrity.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, social network, or retention-driven product.
- What the repo appears to be moving toward: Strongly implied by code/docs. Continued `app_core` migration, stronger GUI automation/catalog contracts, tighter guardrails, and smaller runtime/native-shell surfaces without reopening broad renderer rewrites.
- What is merely possible but unsupported: Weakly implied / uncertain. Promoting desktop AIV into CI now, reviving dormant browser-column chips, or broad redesign of the legacy controller are not justified by the current repository evidence.

## Audit Baseline

- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` passed during audit startup; branch policy, migration-boundary guardrails, public/private docs guardrails, and the agent-safe validation lane are currently green.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` currently fails with `23` non-allowlisted over-budget Rust files.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md` on `2026-04-01`; the live tree now shows `30` over-budget Rust files total, `3` files with `dead_code` suppressions, and `3` files with `clippy::too_many_arguments` suppressions.
- `docs/QUALITY_SCORE.md` still cites stale exact counts even though `scripts/check_quality_score_drift.ps1` passes, because the current drift check only enforces the score row, not the narrative counts.
- The current full-scan file-size debt is split between:
  - non-allowlisted production/runtime files such as `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_toolbar.rs`, `vendor/radiant/src/gui/layout_core/engine/context.rs`, `vendor/radiant/src/gui_runtime/native_vello/input.rs`, `vendor/radiant/src/gui_runtime/native_vello/runtime_events/keyboard.rs`, and `src/selection/range.rs`
  - oversized test hubs and inline-test files such as `src/app/controller/tests/browser_core/marks.rs`, `src/app_core/native_bridge/tests/bridge_runtime/projection.rs`, `vendor/radiant/src/gui/native_shell/state/automation.rs`, and `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/worker.rs`

## Ordered Backlog

### 1. [x] Resolve the public waveform-shift action contract so cataloged actions cannot silently no-op

- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: S-M
- Why it matters: the host-owned GUI action catalog currently publishes waveform shift-arm actions that the migration-facing dispatcher does not handle, so a public action id can exist in docs/tests while doing nothing when dispatched through `app_core`.
- Evidence:
  - `src/app_core/actions/catalog/entries.rs` publishes `begin_waveform_selection_shift` and `begin_waveform_edit_selection_shift`.
  - `src/app_core/controller.rs` routes transport/browser/map/waveform/prompt-update groups, then drops any remaining action through the generic unhandled path.
  - `src/app_core/controller/waveform_actions.rs` returns `Err(action)` for any unmatched waveform action.
  - `vendor/radiant/src/gui_runtime/native_vello/input/waveform_routing/press.rs` creates these actions as press-only drag arms, while `vendor/radiant/src/gui_runtime/native_vello/runtime_input/drag.rs` explicitly treats them as non-emitting pointer-arm actions.
- Recommended change: make the public contract truthful by either routing these actions to real controller behavior or removing/downgrading them from the host catalog and coverage surfaces if they are runtime-internal gesture arms rather than dispatchable host API.
- Expected impact: removes a concrete catalog/dispatcher mismatch and prevents future hosts or tests from relying on inert public actions.
- Risks / tradeoffs: medium. The safe fix depends on whether these actions are intended as public host API or runtime-internal drag setup.
- Dependencies: none
- Suggested validation:
  - targeted `src/app_core/controller/tests` coverage for whichever contract is chosen
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Date completed: `2026-04-01`
- Commit: pending until the first Phase 2 item-1 commit is created
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Assumptions used:
  - The host-owned public dispatch surface should only include actions that are meaningful to dispatch directly through `app_core`.
  - `BeginWaveformSelectionShift` and `BeginWaveformEditSelectionShift` remain valid runtime-internal gesture-arm actions for `radiant`, but not public GUI runner inputs.
- Execution notes:
  - Added explicit catalog dispatch policy and marked the two waveform shift-arm actions as `runtime_internal`.
  - Updated the in-process GUI runner to reject runtime-internal actions explicitly instead of dispatching them into the unhandled path.
  - Updated `docs/gui_test_platform.md` so the catalog contract distinguishes exhaustive coverage from public-dispatch support.

### 2. [ ] Make GUI action-trace assertions distinguish handled behavior from mere dispatch attempts

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the current GUI scenario/AIV layers can stay green after an unhandled or ineffective action because `ActionRecorded` only proves that the action id was appended to the trace after dispatch was attempted.
- Evidence:
  - `src/gui_test/runner/mod.rs` appends `trace_event_for_action(...)` immediately after `bridge.reduce_action(...)`.
  - `src/app_core/native_bridge/mod.rs` records an action trace after `reduce_action(...)` completes, regardless of whether the controller handled it meaningfully.
  - `src/app_core/native_bridge/gui_test.rs` persists the same post-dispatch trace into live artifacts.
  - `src/gui_test/runner/assertions.rs` implements `ActionRecorded` as a plain action-id presence check.
  - `src/gui_test/packs/transport.rs` and `src/gui_test/aiv/packs/cases/waveform.rs` rely on `ActionRecorded` for coverage.
- Recommended change: record handled/result status in the GUI trace or replace coverage-critical `ActionRecorded` assertions with state-based assertions for actions whose observable effects matter.
- Expected impact: prevents false-green GUI contract coverage and makes action-catalog regressions easier to detect early.
- Risks / tradeoffs: medium. Tightening this contract may expose pre-existing weak assertions that need to be rewritten.
- Dependencies: item 1 benefits from this, but it is independently valuable
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - targeted `src/gui_test/runner/tests.rs`
  - one focused live-artifact assertion path through `src/app_core/native_bridge/tests`
- Product clarification required: No

### 3. [ ] Collapse the duplicated keyboard-routing paths in `vendor/radiant` so tests and production execute the same logic

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: deterministic keyboard behavior is a documented product requirement, but the native runtime currently maintains separate test-only and production keyboard paths for the same enter/escape/text/hotkey behavior.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_events/keyboard.rs` defines `handle_hotkey_press_for_tests`, `handle_character_key_for_tests`, `handle_enter_for_tests`, and `handle_escape_for_tests`.
  - The same file re-implements the same escape/enter/text/hotkey branches inside `handle_keyboard_input(...)`.
  - Focused key-binding tests under `vendor/radiant/src/gui_runtime/native_vello/tests/key_bindings/` call the test helpers, while the real runtime uses `handle_keyboard_input(...)`.
  - `docs/design_principles.md` explicitly treats keyboard determinism and Esc semantics as first-class behavior.
- Recommended change: extract shared helpers for enter/escape/text-input/hotkey routing and make both tests and production route through that single branch logic.
- Expected impact: reduces drift risk in one of the most focus-sensitive runtime paths and removes one current non-allowlisted runtime file-size violation.
- Risks / tradeoffs: medium. Keyboard behavior is user-facing and regression-sensitive, so refactoring must preserve exact focus, prompt, and text-target semantics.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test --manifest-path vendor/radiant/Cargo.toml key_bindings`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 4. [ ] Finish the `app_core` dispatch-hub split so migration-facing routing depends on narrower controller seams

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: `app_core` is still a migration-facing boundary, but the browser and waveform dispatch helpers remain large branch-heavy matchers that directly depend on legacy controller state layout.
- Evidence:
  - `src/app_core/controller/browser_actions.rs` keeps `apply_browser_native_ui_action(...)` as a broad browser/source/folder/options/drag matcher.
  - `src/app_core/controller/waveform_actions.rs` keeps `apply_waveform_native_ui_action(...)` as a broad waveform/edit/zoom/slice/destructive matcher.
  - `tmp/cleanup_audit_hotspots.md` reports function spans of `254` lines for `apply_browser_native_ui_action` and `279` lines for `apply_waveform_native_ui_action`.
  - `src/app_core/controller.rs` is still a type alias over the legacy controller, and helpers such as `src/app_core/controller/waveform_actions.rs` and `src/app_core/controller/prompt_update_actions.rs` still mutate `controller.ui` state directly.
  - Focused tests exist in `src/app_core/controller/tests/dispatch/`, `src/app_core/controller/tests/browser_sources/`, and `src/app_core/controller/tests/waveform/`, but those tests still validate broad route groups rather than narrow helper contracts.
- Recommended change: continue the split from route-group files into smaller surface-specific helpers and replace direct `controller.ui` field mutations with narrower controller methods where a clear seam already exists.
- Expected impact: lowers maintenance cost for migration work and makes future routing regressions easier to localize without reopening a broad controller rewrite.
- Risks / tradeoffs: medium. These routes are compatibility-sensitive and still sit on top of legacy internals.
- Dependencies: item 1 should be resolved first because it changes the actionable waveform contract
- Suggested validation:
  - targeted `src/app_core/controller/tests`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
- Product clarification required: No

### 5. [ ] Burn down the remaining non-allowlisted production/runtime file-size debt before touching explicitly allowlisted exceptions

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: L
- Why it matters: the full-scan file-size lane is still red, and the remaining non-allowlisted production debt sits in active runtime/layout/native-shell surfaces where smaller modules would improve both guardrail health and day-to-day reasoning.
- Evidence:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` currently reports `23` non-allowlisted violations.
  - The live non-allowlisted production/runtime backlog includes:
    - `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_toolbar.rs` (`421`)
    - `vendor/radiant/src/gui/layout_core/engine/context.rs` (`413`)
    - `vendor/radiant/src/gui_runtime/native_vello/input.rs` (`403`)
    - `vendor/radiant/src/gui_runtime/native_vello/runtime_events/keyboard.rs` (`403`)
    - `src/selection/range.rs` (`407`)
  - `docs/file_size_budget_allowlist.txt` already preserves intentional exceptions such as `src/app_core/actions/catalog/kinds.rs` and `src/app/controller/playback/transport/selection.rs`, so the current red lane is concentrated in files that are not yet documented as acceptable exceptions.
- Recommended change: split the non-allowlisted production/runtime files in production-first order, folding `runtime_events/keyboard.rs` into item 3 and treating `src/selection/range.rs` conservatively unless a clearer sub-boundary emerges than the module’s current cohesion note allows.
- Expected impact: restores guardrail credibility around active runtime code and reduces review/maintenance friction in the native shell and input layers.
- Risks / tradeoffs: medium. Behavior-preserving structural work in runtime/input/layout code is regression-sensitive.
- Dependencies: item 3 should be completed first for `runtime_events/keyboard.rs`
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted runtime/native-shell tests for each split cluster
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 6. [ ] Split the oversized test hubs and inline-test modules so the file-size policy remains credible

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: L
- Why it matters: a large share of the remaining budget debt is test code or inline test bulk, which makes failures harder to localize and weakens the meaning of the small-file policy.
- Evidence:
  - Current oversized test hubs include:
    - `src/app/controller/tests/browser_core/marks.rs` (`439`)
    - `src/app/controller/playback/recording/waveform_loader/tests.rs` (`432`)
    - `src/app/controller/tests/browser_core/filters.rs` (`430`)
    - `src/sample_sources/db/file_ops_journal/tests.rs` (`423`)
    - `src/app/controller/tests/waveform_cache_loading.rs` (`422`)
    - `src/analysis/ann_index_tests.rs` (`422`)
    - `src/app_core/native_shell/tests/overlays.rs` (`409`)
    - `src/app_core/native_bridge/tests/bridge_runtime/projection.rs` (`409`)
    - `src/app/controller/tests/browser_selection.rs` (`409`)
    - `src/sample_sources/scanner/scan/tests.rs` (`407`)
    - `vendor/radiant/src/gui_runtime/native_vello/tests/browser_pointer/surface_routes.rs` (`412`)
    - `vendor/radiant/src/gui/native_shell/state/tests/browser_rows/virtualization.rs` (`406`)
    - `vendor/radiant/src/gui/native_shell/state/tests/overlays/waveform_hover.rs` (`403`)
  - Some red files are production modules only because large inline test blocks live in them, for example `vendor/radiant/src/gui/native_shell/state/automation.rs` and `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/worker.rs`.
- Recommended change: split the largest test hubs by behavior or fixture family and move large inline test modules beside dedicated test files where that removes false production-file debt.
- Expected impact: improves test discoverability, makes failures easier to route, and lets the file-size budget report focus more truthfully on production debt.
- Risks / tradeoffs: medium. Mechanical moves can still damage fixture readability or helper reuse if done carelessly.
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
- Why it matters: the score row is kept honest, but the surrounding narrative still cites stale exact counts, which weakens the scorecard as a wake-up and prioritization artifact.
- Evidence:
  - `docs/QUALITY_SCORE.md` still says the live tree has `25` non-allowlisted over-budget Rust files and `2` files with `dead_code` suppressions.
  - The refreshed `tmp/cleanup_audit_hotspots.md` now reports `30` over-budget Rust files total and `3` files with `dead_code` suppressions.
  - `scripts/check_quality_score_drift.ps1` only validates the score row against the high-visibility guardrail state.
- Recommended change: either make the drift check validate the cited counts against the current audit output, or rewrite the narrative notes to avoid brittle exact numbers that are expected to change frequently.
- Expected impact: keeps the scorecard truthful without forcing humans to rediscover which parts of it are enforced versus advisory.
- Risks / tradeoffs: low. Stronger validation increases maintenance cost slightly; looser narrative wording reduces precision.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
- Product clarification required: No

### 8. [ ] Reconcile shell-specific validation/doc-tooling drift so the same named workflow does not imply different guarantees

- Classification: Developer-experience improvement
- Confidence: Medium-High
- ROI: Medium
- Effort: S-M
- Why it matters: this repository documents shared Bash/PowerShell workflows, but several script pairs currently differ enough to create quiet churn or misleading expectations depending on shell.
- Evidence:
  - `scripts/ci_quick.ps1` runs `scripts/run_gui_contract.ps1`, while `scripts/ci_quick.sh` runs only the quick `nextest` lane.
  - `scripts/ci_local.ps1` describes itself as CI-parity, but the Windows path skips CI advisory steps such as the dead-dependency sweep and env-var drift report that appear in `.github/workflows/ci.yml`.
  - `scripts/check_manual_docs_scope.ps1` and `.sh` allow more manual redirect stubs than their failure output advertises.
  - `scripts/fix_trivial_doc_links.ps1` rewrites `manual/plan.md` and `manual/todo.md` to legacy `docs/plans/*.md` paths, while `scripts/fix_trivial_doc_links.sh` rewrites to `docs/plans/active/*.md`.
- Recommended change: either align the shell pairs to the same behavior or narrow the docs/help text and failure output so each script truthfully describes its own guarantees and canonical rewrite targets.
- Expected impact: reduces cross-shell confusion and avoids needless doc churn from maintenance helpers.
- Risks / tradeoffs: low. The main tradeoff is whether the repo wants behavior parity or intentionally shell-specific contracts with clearer wording.
- Dependencies: none
- Suggested validation:
  - targeted script fixture coverage where possible
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1`
  - spot-check help output for the affected wrappers
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. Which `app_core` GUI actions are intended to be stable, externally dispatchable host API rather than runtime-internal gesture arms?

- Evidence:
  - `src/app_core/actions/catalog/entries.rs` publishes `begin_waveform_selection_shift`, `begin_waveform_edit_selection_shift`, `play_from_start`, and `commit_volume_setting` as host catalog entries.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_input/drag.rs` treats the waveform-shift actions as non-emitting pointer-arm actions.
  - `src/app_core/controller.rs` flushes pending volume persistence every frame and also exposes `CommitVolumeSetting` as an explicit public action.
- Why this matters: items 1 and 2 depend on knowing whether the public host surface should contain gesture-arm and commit-detail actions or only externally meaningful state transitions.
- Affected files/modules:
  - `src/app_core/actions/catalog/entries.rs`
  - `src/app_core/controller.rs`
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_input/drag.rs`
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_input/finish.rs`
- Risk if guessed incorrectly: the repo could either keep publishing inert actions or remove surface that native hosts/AIV were meant to target explicitly.
- Most conservative provisional assumption: keep only actions whose dispatch has meaningful controller-side semantics, and treat press-only gesture-arm actions as internal until explicitly documented otherwise.

### [!] 2. Should stable action identifiers remain mirrored across the host catalog and `radiant` automation layer, or is a shared lower-layer contract intended?

- Evidence:
  - `docs/gui_test_platform.md` says the host action catalog in `src/app_core/actions` is canonical.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs` still keeps a manual `UiAction` to slug matcher.
  - `vendor/radiant/src/gui/native_shell/state/automation/*.rs` also hardcode several `available_actions` strings.
  - `src/gui_test/runner/tests.rs` guards that advertised action ids are cataloged, but not that every automation-advertised action matches the click path for the same control.
- Why this matters: safe cleanup of the remaining automation duplication depends on whether the boundary should stay split with tests as the safety net, or whether the repo wants a shared lower-layer action-id source.
- Affected files/modules:
  - `docs/gui_test_platform.md`
  - `src/app_core/actions/catalog/entries.rs`
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs`
  - `vendor/radiant/src/gui/native_shell/state/automation/*.rs`
- Risk if guessed incorrectly: a cleanup could either violate the intended `src`/`vendor` ownership boundary or preserve needless duplication longer than necessary.
- Most conservative provisional assumption: preserve the current boundary split and strengthen parity tests before attempting any shared-source refactor.

### [!] 3. What exact behavior should the public `PlayFromStart` and `CommitVolumeSetting` contracts promise to hosts and tests?

- Evidence:
  - `src/app_core/actions/catalog/kinds.rs` documents `PlayFromStart` as starting from the beginning of the current sample or loop.
  - `src/gui_test/packs/transport.rs` names the scenario `transport_play_from_selection_start`.
  - `src/app/controller/playback/transport/playback.rs` explicitly prefers the active playback-selection start over file start.
  - `src/app/controller/playback/transport/volume.rs` persists volume via debounce, while `src/app_core/controller.rs` also exposes explicit `CommitVolumeSetting` and flushes pending volume settings during frame prep.
- Why this matters: future host integrations and coverage updates need one clear contract for playback restart and volume persistence rather than multiple slightly different implied meanings.
- Affected files/modules:
  - `src/app_core/actions/catalog/kinds.rs`
  - `src/gui_test/packs/transport.rs`
  - `src/app/controller/playback/transport/playback.rs`
  - `src/app/controller/playback/transport/volume.rs`
- Risk if guessed incorrectly: hosts may emit the wrong action sequence or tests may lock in accidental behavior as a public API promise.
- Most conservative provisional assumption: preserve current controller behavior exactly and update docs/tests to match only after the intended public contract is made explicit.

### [!] 4. Is `src/selection/range.rs` supposed to stay as one cohesive domain exception, or should the file-size policy force a split there too?

- Evidence:
  - `src/selection/range.rs` is currently `407` lines and fails the live file-size budget.
  - The module-level docs in `src/selection/range.rs` explicitly say the preferred maintenance approach is to preserve cohesion unless a clearly separate subdomain emerges.
  - `docs/file_size_budget_allowlist.txt` does not currently exempt `src/selection/range.rs`, unlike `src/app/controller/playback/transport/selection.rs`, which carries a similar cohesion rationale.
- Why this matters: item 5 needs to know whether `SelectionRange` is real active debt or an undocumented intentional exception.
- Affected files/modules:
  - `src/selection/range.rs`
  - `docs/file_size_budget_allowlist.txt`
  - `scripts/check_file_size_budget.*`
- Risk if guessed incorrectly: the repo could either split a deliberately cohesive domain model unnecessarily or keep a known policy contradiction unresolved.
- Most conservative provisional assumption: do not split `SelectionRange` unless a real sub-boundary emerges; if it remains cohesive, document that intent explicitly.

## Rejected Ideas

### [-] 1. Split the explicitly allowlisted centralized tables just to satisfy the file-size budget

- Why it was considered: `src/app_core/actions/catalog/kinds.rs` and `src/app/controller/playback/transport/selection.rs` remain among the largest files in the tree.
- Why it was rejected: `docs/file_size_budget_allowlist.txt` explicitly justifies both as intentional centralized/cohesive exceptions, so raw size alone is not enough evidence to make them the next lane.
- What evidence was missing: concrete correctness, ownership, or discoverability failures caused by their current centralization.

### [-] 2. Revive browser-column chips because render and hit-test scaffolding still exists

- Why it was considered: `vendor/radiant/src/gui/native_shell/state/frame_build/browser/panel.rs` still renders column chips when provided, and related `SelectColumn` / `MoveColumn` actions still exist.
- Why it was rejected: `vendor/radiant/src/gui/native_shell/tests/toolbar.rs` explicitly asserts that browser column chip hit targets are absent today, and the current user docs do not treat column chips as an active UI feature.
- What evidence was missing: explicit product documentation or an active runtime path showing that visible column chips are intended now rather than dormant scaffolding.

### [-] 3. Promote desktop AIV into CI or make it the primary validation lane now

- Why it was considered: the repo has broader desktop-AIV packs and dedicated PowerShell wrappers.
- Why it was rejected: `docs/gui_test_platform.md` still documents desktop AIV as local-only because Windows foreground activation remains unstable.
- What evidence was missing: repeated stability data and an explicit documented promotion bar for CI.
