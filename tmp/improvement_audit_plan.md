# Improvement Audit Plan

Generated: 2026-03-29
Observed superproject commit: `148839fd`
Observed `vendor/radiant` commit: `091d1674`
Observed workspace state: dirty worktree in both repos; findings below reflect the live workspace seen during this audit.
Status: Phase 2 execution in progress on 2026-03-29; items 1-3 and 7 are completed, items 4-6 and 8-9 are clarification-gated or blocked, and item 10 is the next safe executable task.

## Scope

- This document supersedes the previous completed execution record that lived at this path.
- Findings are ranked in strict execution order by expected ROI for the current workspace, not by category.
- Recommendations stay inside repository-supported direction. Broad rewrites, speculative features, and preference-only cleanup are excluded.

## Decision Log

- 2026-03-29: This is a fresh Phase 1 evidence-driven audit of the current workspace, not a continuation of the 2026-03-25 execution record.
- 2026-03-29: The current workspace is dirty, so evidence that depends on line counts or guardrail status applies to the live tree observed now, not necessarily to the last clean commit.
- 2026-03-29: The user explicitly approved Phase 2 sequential implementation of this backlog.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as an early-alpha Rust desktop tool for triaging, auditioning, editing, and curating local audio samples.
- Maturity level: Explicitly documented. `README.md` warns that the app is early alpha and can modify, rename, or delete sample-library files.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace with the root `sempal` crate, support apps/tools, and the vendored `radiant` GUI/runtime submodule.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` splits domain/controller logic under `src/`, GUI/runtime behavior under `vendor/radiant/`, host projections under `src/app_core`, and support binaries under `apps/` and `tools/`.
- Architectural boundaries: Explicitly documented. `README.md` and `docs/ARCHITECTURE.md` keep domain state and UI intent in `src`, while `vendor/radiant` owns widget behavior, layout, input routing, and rendering coordination.
- Test strategy: Strongly implied by code/docs. `docs/TEST.md` and `.github/workflows/ci.yml` center the repo on deterministic Rust unit/module tests, `cargo nextest`, targeted GUI contract tests, and optional desktop-AIV loops.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, non-blocking execution, predictability, reversibility, and data integrity.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, social network, or attention-retention product.

## Audit Notes

- `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1` passed.
- `powershell -ExecutionPolicy Bypass -File scripts/check_codeowners_coverage.ps1` passed.
- `powershell -ExecutionPolicy Bypass -File scripts/check_rust_taste_invariants.ps1` passed, but the dirty Windows worktree emitted repeated Git CRLF warnings while diff-aware scripts ran.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` failed on the live tree with 30 over-budget files across `src/` and `vendor/radiant/src`, including production files such as `src/app/controller/library/wavs/audio_loading.rs`, `src/app/controller/library/wavs/entry_mutation.rs`, and several non-test `vendor/radiant` modules.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` did not report guardrail state cleanly on the live tree because a Git line-ending warning was surfaced as a terminating PowerShell error before the wrapper could classify pass/fail.
- `tmp/cleanup_audit_hotspots.md` is stale relative to the live workspace and was generated from a script path that still ignores `vendor/radiant` submodule files when building the hotspot snapshot.

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for listening to, navigating, editing, and curating local sample libraries with strong emphasis on responsiveness and reversible workflows.
- What the repo appears to be moving toward: Strongly implied by code/docs. Tighter stateless-agent guardrails, better GUI contract coverage, continued `app_core`/runtime separation, and more reliable Windows validation wrappers.
- What is merely possible but unsupported: broad `app_core` redesigns, replacing the vendored runtime strategy, or promoting unstable desktop AIV coverage into default CI now.

## Ordered Backlog

### 1. [x] Make `scripts/check_quality_score_drift.ps1` robust to benign Git warning output on Windows

- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the PowerShell drift wrapper is part of the documented Windows guardrail path, but it can terminate before reporting any real PASS/FAIL result when a nested check writes a non-fatal Git warning to stderr.
- Evidence:
  - `scripts/check_quality_score_drift.ps1:18-19` sets `$ErrorActionPreference = "Stop"`.
  - `scripts/check_quality_score_drift.ps1:41-70` wraps nested script execution in `Invoke-GuardrailCheck`.
  - `scripts/check_quality_score_drift.ps1:59` captures child output via `& $psExe @args 2>&1`.
  - On the current tree, running `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` aborted at line 59 after Git emitted `LF will be replaced by CRLF`, so the wrapper never reached its own guardrail-status classification.
  - The bash variant in `scripts/check_quality_score_drift.sh:59-87` handles nested command status explicitly and does not have this failure mode.
- Recommended change: change the PowerShell wrapper so nested-script stderr warnings do not become terminating wrapper errors; preserve child exit code as the source of truth and print warnings as diagnostic output instead of aborting classification.
- Expected impact: restores trust in the documented Windows guardrail check and removes a false-negative failure mode during normal agent use.
- Risks / tradeoffs: low; the main risk is masking real wrapper errors if stderr handling becomes too blunt.
- Dependencies: none
- Suggested validation:
  - reproduce with a warning-producing dirty tree and confirm the script reports the nested guardrail result instead of aborting
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-29
- Commit: `4c94dac5`
- Assumptions: benign child-process stderr should remain visible as diagnostics, while the child exit code remains the source of truth for PASS/FAIL classification.
- Validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1 2>&1 | Out-String`
    now reaches nested guardrail classification instead of aborting on Git CRLF warnings; the remaining failure is the expected stale-score mismatch tracked by item 3
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Plan order deviation: none

### 2. [x] Repair the cleanup-hotspot audit scripts so ROI planning includes `vendor/radiant`

- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: S-M
- Why it matters: the repo explicitly documents `scripts/audit_cleanup_hotspots.*` as the input for cleanup/ROI planning, but the current implementation silently omits the vendored GUI/runtime submodule even though the repo's workflow and file-size budgeting still treat it as live development code.
- Evidence:
  - `docs/INDEX.md:33-35` recommends `scripts/audit_cleanup_hotspots.*` for ROI planning.
  - `scripts/audit_cleanup_hotspots.sh:123` enumerates Rust files via `git ls-files '*.rs'`.
  - `scripts/audit_cleanup_hotspots.ps1:35-38` uses the same superproject-only enumeration in `Get-RustFiles`.
  - The current snapshot at `tmp/cleanup_audit_hotspots.md` reports 922 Rust files scanned and misses much larger live files under `vendor/radiant`.
  - A direct scan of the current tree found `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs` at 1636 lines and `vendor/radiant/src/app/hotkeys.rs` at 875 lines, neither of which can appear in the current hotspot snapshot.
- Recommended change: teach both hotspot audit scripts to enumerate `vendor/radiant` Rust files from the initialized submodule checkout, with a filesystem fallback when the nested repo is present but not fully tracked from the superproject.
- Expected impact: future ROI planning will stop undercounting the largest file-size and test-debt hotspots in the actual workspace.
- Risks / tradeoffs: low to medium; the main risk is double-counting or producing inconsistent output when the submodule is missing or detached.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
  - inspect `tmp/cleanup_audit_hotspots.md` and confirm `vendor/radiant` files appear in the largest-file and over-budget sections
  - compare the refreshed snapshot against `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
- Product clarification required: No
- Completed: 2026-03-29
- Commit: `c46af560`
- Assumptions: the vendored `vendor/radiant` checkout remains intentional live development code for cleanup planning, so the hotspot snapshot should enumerate it whenever the nested repo or working tree is present.
- Validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
  - `tmp/cleanup_audit_hotspots.md` now reports 1254 scanned Rust files and surfaces `vendor/radiant` entries in the largest-file, over-budget, and heuristic hotspot sections
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
    still reports the live 30 file-budget violations, including the `vendor/radiant` cluster that the refreshed hotspot snapshot now exposes
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Plan order deviation: none

### 3. [x] Re-baseline `docs/QUALITY_SCORE.md` against the live file-size state

- Classification: Documentation gap
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the scorecard is supposed to keep high-visibility guardrail health obvious to future agents, but it currently claims a green full-scan file-size state that no longer matches the observed workspace.
- Evidence:
  - `docs/QUALITY_SCORE.md:24` says `Agent-facing guardrails` are at score `4` and that the full file-size scan is green again.
  - `docs/QUALITY_SCORE.md:26` says the enforced full-scan budget passes with two documented cohesive exceptions.
  - `docs/file_size_budget_allowlist.txt` currently lists only seven allowlisted paths.
  - The live `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` run reported 30 over-budget files across the root repo and `vendor/radiant`.
- Recommended change: update `docs/QUALITY_SCORE.md` so the guardrail and code-size rows describe the current observed state honestly, including whether the claims apply to the live dirty workspace or only to the last clean baseline.
- Expected impact: future audits and handoffs start from truthful guardrail posture instead of stale "green again" language.
- Risks / tradeoffs: low; the main risk is overfitting the scorecard to temporary local WIP unless the wording names the observed scope clearly.
- Dependencies: items 1 and 2 make the supporting guardrail/reporting path more trustworthy.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
  - rerun `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` and confirm the scorecard describes the observed result honestly
- Product clarification required: No
- Completed: 2026-03-29
- Commit: `afb61db3`
- Assumptions: the scorecard should describe the live observed dirty workspace explicitly rather than silently reusing the last clean-baseline claims.
- Validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
    now passes with a degraded guardrail posture and score `3`, instead of failing on stale healthy wording
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Plan order deviation: none

### 4. [!] Clarify whether compare-anchor state is part of the undoable "meaningful UI" contract

- Classification: Product-definition gap
- Confidence: High
- ROI: High
- Effort: S-M
- Why it matters: the design principles promise uniform undo/redo coverage for meaningful in-session curation workflows, but compare-anchor state currently mutates outside that contract and its intended reversibility is undocumented.
- Evidence:
  - `docs/design_principles.md:126-134` says meaningful in-session workflows should be reversible via undo/redo.
  - `src/app/controller/playback/compare_anchor.rs:7-38` and `:129-188` mutate compare-anchor state directly.
  - `src/app/controller/history.rs:46-80` defines `MeaningfulUiSnapshot`, but it does not include compare-anchor state.
  - `src/app/controller/tests/compare_anchor.rs:21-212` covers set/play/missing-anchor behavior, but not undo/redo.
- Recommended change: decide whether compare-anchor is meaningful undo state; if yes, add it to the snapshot/restore path and cover it with history tests, and if not, document the explicit exemption in the design/behavior docs.
- Expected impact: aligns a live curation feature with the repo's stated undo model and removes an ambiguity that could otherwise cause user-trust regressions.
- Risks / tradeoffs: medium; treating it as undoable broadens snapshot churn, while exempting it weakens the "uniform undo" story.
- Dependencies: none
- Suggested validation:
  - focused compare-anchor/history undo tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: Yes

### 5. [~] Deepen regression coverage for `MeaningfulUiSnapshot` restore and async history completion paths

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the meaningful-UI history path restores a wide surface of browser, waveform, folder, and async-completion state, but current regression coverage only exercises a narrow subset of that contract.
- Evidence:
  - `src/app/controller/history.rs:98-109` wraps mutations in `record_meaningful_ui_transaction`.
  - `src/app/controller/history.rs:167-252` restores selected source, browser selection, `loaded_wav`, waveform selection/edit selection, view, cursor, and loop state.
  - `src/app/controller/history.rs:350-419` finalizes async overwrite/creation transactions and attaches UI restore hooks.
  - `src/app/controller/tests/history_transactions.rs:1-113` only covers four basic undo/redo cases.
  - `src/app/controller/library/selection_export/selection_export_tests/waveform_selection_export_tests.rs:147-454` covers one creation-success lane and one failure lane, but not overwrite completion or the fuller snapshot-restore surface.
- Recommended change: add focused history tests for capture/restore of source/browser/folder selection, waveform state, and async completion hooks, using table-driven snapshots instead of one large scenario file.
- Expected impact: tighter protection for one of the repo's core reversibility contracts without changing design direction.
- Risks / tradeoffs: medium; the tests need disciplined fixtures so they validate behavior rather than internal representation details.
- Dependencies: item 4 if compare-anchor is judged part of the snapshot contract
- Suggested validation:
  - targeted history and selection-export tests in one cargo process
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Blocked: 2026-03-29
- Blocking dependency: item 4 compare-anchor clarification
- Blocking note: safe coverage for the snapshot contract depends on whether compare-anchor must participate in undo/redo, so this test expansion is deferred until that contract is defined.

### 6. [!] Define the lifecycle for retained pending-rename rows in the source DB

- Classification: Product-definition gap
- Confidence: Medium
- ROI: High
- Effort: M
- Why it matters: pending rename rows preserve tags and metadata for quick/deep rename reconciliation, but the repo does not currently define how long unmatched rows should survive or when they should be pruned.
- Evidence:
  - `src/sample_sources/scanner/scan_diff.rs:123-128` stages every leftover missing row as a pending rename during quick scans.
  - `src/sample_sources/db/pending_renames.rs:99-228` only clears rows when they are claimed or when a live-path upsert conflicts with them.
  - `src/sample_sources/scanner/scan_hash.rs:21-120` only reconciles and clears rows when deep-hash matching succeeds.
  - `src/sample_sources/scanner/scan/tests.rs:258-289` intentionally leaves ambiguous large-file renames in `pending_wav_renames`.
  - `src/sample_sources/scanner/scan/runner.rs:81-87` documents hard rescans as moving rename work into the deeper pass, but there is no documented policy for stale unmatched pending rows.
- Recommended change: document one explicit retention/pruning policy for pending renames, then enforce it in the scanner/DB helpers and add tests for hard-rescan, ambiguous-rename, and eventual-prune behavior.
- Expected impact: removes a silent trust-model ambiguity around whether metadata for deleted/moved samples is preserved temporarily or indefinitely.
- Risks / tradeoffs: medium; an aggressive prune policy can lose intended metadata preservation, while indefinite retention can accumulate stale rows and surprising future matches.
- Dependencies: none
- Suggested validation:
  - targeted scanner/db tests for quick scan, deep scan, ambiguous rename, and prune behavior
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: Yes

### 7. [x] Add mutation-invariant coverage for `entry_mutation.rs`, including cache, DB, and compare-anchor updates

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: S-M
- Why it matters: rename/move helpers are part of the repo's trust model. This layer rewrites database rows, lookup caches, selection paths, and compare-anchor paths, but most of those invariants are still only covered indirectly.
- Evidence:
  - `src/app/controller/library/wavs/entry_mutation.rs:31-74` rewrites DB rows and preserves metadata across path changes.
  - `src/app/controller/library/wavs/entry_mutation.rs:208-338` updates cached lookups, selection paths, compare-anchor paths, and UI invalidation state.
  - `src/app/controller/library/wavs/entry_mutation.rs:383-436` only tests numbered-name suggestion helpers.
  - A repository-wide search shows this helper is called from file ops, waveform slide, drag/drop, selection export, duplicate cleanup, background polling, and browser-controller delegates.
- Recommended change: add focused tests for metadata rewrite, lookup/cache updates, compare-anchor path rewrites, and selection-path propagation during rename/move operations.
- Expected impact: hardens a data-integrity-sensitive helper layer with cheaper, more local coverage than relying only on high-level UI-driven tests.
- Risks / tradeoffs: low; the main risk is over-coupling tests to implementation details instead of invariant-level outcomes.
- Dependencies: item 4 only if compare-anchor semantics change materially
- Suggested validation:
  - targeted controller/browser helper tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed: 2026-03-29
- Commit: pending (record after commit)
- Assumptions: compare-anchor rename-path rewrites are still safe to validate even while the separate undo/redo contract question remains unresolved.
- Validation:
  - `cargo test entry_mutation_tests --lib -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Plan order deviation: none

### 8. [!] Make `GuiScenarioStep::CaptureSnapshot` truthful or remove it from the supported scenario contract

- Classification: Bug fix
- Confidence: High
- ROI: Medium-High
- Effort: S-M
- Why it matters: the public GUI-scenario schema exposes a labeled capture step, but the runner currently treats that step as a silent no-op, which means the supported contract is misleading for CLI users and future automation consumers.
- Evidence:
  - `src/gui_test/scenario.rs:20-24` defines `GuiScenarioStep::CaptureSnapshot { label }`.
  - `src/gui_test/runner.rs:66` currently handles `GuiScenarioStep::CaptureSnapshot { .. }` with an empty arm.
  - `src/gui_test/artifacts.rs:80` only stores one final `automation_snapshot`, so there is nowhere for intermediate labeled captures to land today.
- Recommended change: either implement labeled intermediate snapshot capture in the artifact/report path or remove/deprecate the step so unsupported behavior is not silently advertised.
- Expected impact: makes the GUI scenario schema honest and prevents future tooling from depending on a no-op feature.
- Risks / tradeoffs: medium; adding intermediate captures expands artifact schema, while removing the step may require a migration path for any unpublished consumers.
- Dependencies: none
- Suggested validation:
  - targeted `src/gui_test` scenario-runner tests for the chosen behavior
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: Yes

### 9. [~] Split `src/gui_test/runner.rs` once the capture-step contract is settled

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: the GUI runner is now an over-budget, multi-responsibility file that mixes fixture bootstrap, scenario execution, assertion logic, artifact assembly, step labeling, and local tests.
- Evidence:
  - The live full scan flags `src/gui_test/runner.rs` at 425 lines.
  - `src/gui_test/runner.rs:27-55` exposes bundle capture/dispatch entrypoints.
  - `src/gui_test/runner.rs:55-100` runs scenarios and timings.
  - `src/gui_test/runner.rs:145-233` contains assertion evaluation and step labeling.
  - `src/gui_test/runner.rs:247-446` embeds a sizeable local test module in the same file.
- Recommended change: split the file around `execution`, `assertions`, and `bundle/artifact` responsibilities after item 8 clarifies whether capture steps remain part of the contract.
- Expected impact: restores the repo's file-size and single-responsibility discipline in one of the actively evolving GUI-test-platform modules.
- Risks / tradeoffs: medium; moving tests and helpers can create temporary churn if the split is not anchored to stable boundaries.
- Dependencies: item 8
- Suggested validation:
  - targeted `cargo test gui_test:: -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
- Product clarification required: No
- Blocked: 2026-03-29
- Blocking dependency: item 8 capture-step contract clarification
- Blocking note: the split boundary for `src/gui_test/runner.rs` depends on whether `CaptureSnapshot` becomes a real artifact feature or is removed from the contract.

### 10. [ ] Add direct command-surface coverage for `gui-test-cli`

- Classification: Test gap
- Confidence: High
- ROI: Medium
- Effort: S-M
- Why it matters: `gui-test-cli` is part of the supported GUI test platform, but most of its command surface still relies on untested top-level branching and argument handling.
- Evidence:
  - `tools/gui-test-cli/src/main.rs:23-104` exposes `snapshot`, `dispatch-action`, `run-scenario`, `run-scenario-pack`, `export-aiv-suite`, and `resolve-node-target`.
  - `tools/gui-test-cli/src/main.rs:128-140` only tests the `export-aiv-suite` argument helper near the bottom of the file.
  - The CLI is referenced throughout `docs/gui_test_platform.md` and `docs/TEST.md` as a normal workflow surface, not just an internal dev script.
- Recommended change: add focused tests for per-command argument validation and at least one smoke path for each supported command, preferably by extracting a small parse/dispatch layer that can be exercised without spawning the full app.
- Expected impact: reduces the chance that a GUI-platform CLI regression is only caught after downstream PowerShell or AIV wrappers fail.
- Risks / tradeoffs: low to medium; end-to-end CLI tests can become noisy if they require too much artifact setup.
- Dependencies: item 8 if `run-scenario` behavior changes with the capture-step contract
- Suggested validation:
  - targeted `cargo test -p gui-test-cli -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
- Product clarification required: No

### 11. [ ] Add direct coverage for the shipped installer entry flow

- Classification: Test gap
- Confidence: Medium
- ROI: Medium
- Effort: S
- Why it matters: the installer binary is built and signed for release, but the top-level branching between uninstall, dry-run, and normal GUI launch is barely tested.
- Evidence:
  - `.github/workflows/release-build.yml:94-101` builds and signs `sempal-installer.exe`.
  - `apps/installer/src/main.rs:22-39` branches on `--uninstall`, `--dry-run`, and the default GUI path.
  - `apps/installer/src/main.rs:41-55` only contains one narrow dry-run plan test.
- Recommended change: extract a small `try_main`/dispatch seam or equivalent so uninstall and default-path branching can be covered without launching the GUI, and keep the existing dry-run plan test as a lower-level helper check.
- Expected impact: protects a shipped release path with low-effort coverage that is currently missing.
- Risks / tradeoffs: low; the main risk is adding test seams that overfit the current top-level structure.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test -p sempal-installer -- --test-threads=1`
  - compare behavior expectations against `.github/workflows/release-build.yml`
- Product clarification required: No

### 12. [ ] Reduce live `vendor/radiant` production file-size debt, starting with the hit-testing and chrome/frame-build cluster

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: L
- Why it matters: the live full scan is red again not only because of test files but also because several active `vendor/radiant` production modules have grown well past the documented 400-line budget.
- Evidence:
  - The live full scan flagged `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome.rs`, `vendor/radiant/src/gui/native_shell/state/hit_testing/waveform.rs`, `vendor/radiant/src/gui/native_shell/layout_adapter/sidebar_header.rs`, `vendor/radiant/src/gui/native_shell/layout_adapter/waveform_annotations.rs`, `vendor/radiant/src/gui/native_shell/state/frame_build/chrome/sidebar_parts/folders.rs`, `vendor/radiant/src/gui/native_shell/state/frame_build/overlay/focus.rs`, and `vendor/radiant/src/gui_runtime/native_vello/text_bpm.rs`.
  - `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome.rs:4-604` currently mixes sidebar rows, folder panel, source actions, options panel, prompt, progress overlay, and top-bar hit testing in one file.
  - `vendor/radiant/src/gui/native_shell/state/frame_build/chrome/sidebar_parts/folders.rs:4-548` currently mixes folder header layout, toggle buttons, inline draft row rendering, row disclosure, labels, and recovery badge rendering.
- Recommended change: split the clustered modules by surface responsibility first (for example sidebar rows vs top-bar/options hit testing, and folder header vs folder-row rendering), then reevaluate the remaining standalone offenders.
- Expected impact: restores code-structure discipline in the live GUI/runtime surface and makes future GUI bugfixes more local and reviewable.
- Risks / tradeoffs: medium; `vendor/radiant` refactors can be mechanically noisy and should avoid changing interaction behavior while splitting files.
- Dependencies: item 2 helps keep follow-up cleanup planning honest
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - targeted `vendor/radiant` tests in one cargo process
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. Should scorecards and wake-up docs describe the live dirty workspace or only the last clean baseline?

- Evidence:
  - `docs/QUALITY_SCORE.md` speaks in present tense about guardrail health.
  - The current workspace is dirty, and the live file-size state is materially worse than the last recorded clean-audit posture.
  - Most guardrails are diff-aware during iteration, which suggests the repo sometimes distinguishes "current live edits" from "clean baseline truth."
- Why this matters: the repo needs one stable rule for what handoff/score docs are allowed to claim, otherwise future sessions can either trust stale green claims or overfit temporary local WIP as baseline truth.
- Affected files/modules:
  - `docs/QUALITY_SCORE.md`
  - `AGENTS.md`
  - `MEMORY.md`
  - `docs/plans/active/todo.md`
- Risk if guessed incorrectly: wake-up docs either hide important live regressions or encode transient local churn as if it were the canonical baseline.
- Most conservative provisional assumption: score and handoff docs should name the observed commit/date and explicitly say when a statement refers to the dirty workspace rather than clean HEAD.

### [!] 2. Should compare-anchor state participate in undo/redo, or is it intentionally exempt from the "meaningful UI" contract?

- Evidence:
  - `docs/design_principles.md` says meaningful curation workflows should be reversible.
  - `src/app/controller/playback/compare_anchor.rs` mutates compare-anchor state directly.
  - `src/app/controller/history.rs` does not currently snapshot or restore that state.
- Why this matters: implementation order for compare-anchor tests and history changes depends on whether the feature is meant to be transient or undoable.
- Affected files/modules:
  - `src/app/controller/playback/compare_anchor.rs`
  - `src/app/controller/history.rs`
  - `src/app/controller/tests/compare_anchor.rs`
- Risk if guessed incorrectly: either undo/redo remains surprisingly incomplete or snapshot churn expands around a state the maintainers intended to keep transient.
- Most conservative provisional assumption: treat compare-anchor as clarification-needed and do not silently widen or narrow the undo contract.

### [!] 3. What is the intended retention/pruning policy for unmatched `pending_wav_renames` rows?

- Evidence:
  - Quick scans stage leftover missing rows.
  - Deep scans only clear rows when a unique match is found.
  - Current tests intentionally allow ambiguous pending rows to remain.
- Why this matters: safe implementation depends on whether the correct outcome is indefinite metadata retention, bounded retention, or explicit prune-on-hard-rescan.
- Affected files/modules:
  - `src/sample_sources/db/pending_renames.rs`
  - `src/sample_sources/scanner/scan_diff.rs`
  - `src/sample_sources/scanner/scan_hash.rs`
  - `src/sample_sources/scanner/scan/tests.rs`
- Risk if guessed incorrectly: either metadata is lost too aggressively or stale rows linger and create surprising future matches.
- Most conservative provisional assumption: keep behavior unchanged until the intended retention policy is documented.

### [!] 4. Should `GuiScenarioStep::CaptureSnapshot` add labeled intermediate snapshots to artifacts, or should the step be removed/deprecated?

- Evidence:
  - The scenario schema exposes the step.
  - The runner currently does nothing for it.
  - The artifact schema currently only stores one final automation snapshot.
- Why this matters: fixing the no-op requires either an artifact/schema expansion or a contract simplification, and the right refactor boundary for `src/gui_test/runner.rs` depends on that decision.
- Affected files/modules:
  - `src/gui_test/scenario.rs`
  - `src/gui_test/runner.rs`
  - `src/gui_test/artifacts.rs`
  - `tools/gui-test-cli/src/main.rs`
- Risk if guessed incorrectly: future tooling depends on a misleading no-op contract or the schema grows in a direction the maintainers do not want.
- Most conservative provisional assumption: unsupported capture steps should fail loudly or remain blocked until the artifact contract is clarified.

## Rejected Ideas

### [-] 1. Split `src/app_core/actions/catalog/kinds.rs` immediately

- Why it was considered: it is currently 543 lines and over the nominal 400-line budget.
- Why it was rejected: `docs/file_size_budget_allowlist.txt` explicitly documents this file as an intentional centralized surface for payload-free GUI action identities and representative-action tooling.
- What evidence was missing: current correctness bugs or ownership pain strong enough to justify breaking the central declaration-order surface.

### [-] 2. Split `src/app/controller/playback/transport/selection.rs` immediately

- Why it was considered: it remains 475 lines and over budget.
- Why it was rejected: the file still reads as one cohesive selection-drag/loop-retarget subdomain and already carries local tests that explain its current behavior.
- What evidence was missing: a concrete bug, duplicated subdomain, or recurring change-friction signal beyond file size alone.

### [-] 3. Replace the small custom CLI parsers with `clap`

- Why it was considered: support binaries in `apps/` and `tools/` still parse some arguments manually.
- Why it was rejected: the current parsers are small, already documented, and at least partially tested; I did not find a repository-specific bug that justifies dependency and migration churn.
- What evidence was missing: a concrete parser correctness issue or clear maintenance failure caused by the current approach.

### [-] 4. Promote the full desktop-AIV suite into normal CI now

- Why it was considered: `docs/gui_test_platform.md` and `docs/plans/active/gui_test_platform_exec_plan.md` show significant desktop-AIV progress.
- Why it was rejected: `docs/gui_test_platform.md` still documents Windows foreground-activation instability as a blocker for CI promotion.
- What evidence was missing: a small stable subset with a documented promotion bar and repeatable success evidence on the current Windows setup.
