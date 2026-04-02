# Improvement Audit Plan

Generated: 2026-04-02
Observed superproject commit: `79fb5c7f`
Observed `vendor/radiant` commit: `75b6d980`
Observed workspace state at audit start: dirty worktree (`MEMORY.md`, `tmp/cleanup_audit_hotspots.md`, several `src/app/**` files, dirty `vendor/radiant`)
Status: Phase 1 audit complete; awaiting explicit user confirmation before any implementation.

## Scope

- This plan audits the current live tree only.
- Findings are ranked in strict execution order by expected ROI for the current repository state.
- Recommendations stay inside documented or strongly implied repository intent.
- No implementation was performed during this audit.
- Broad rewrites, speculative features, and preference-only cleanup are excluded.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as an early-alpha Rust desktop tool for triaging, auditioning, editing, and curating local audio samples with a listening-first workflow.
- Maturity level: Explicitly documented. `README.md` warns that the app is early alpha and can modify, rename, or delete user files.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace centered on the root `sempal` crate, companion apps/tools, and the vendored `radiant` GUI/runtime crate.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` splits domain/controller logic across `src/`, migration-facing projections/actions under `src/app_core`, runtime bridge code under `src/gui_runtime`, GUI behavior under `vendor/radiant`, and support tooling under `apps/` and `tools/`.
- Architectural boundaries: Explicitly documented. `README.md`, `docs/ARCHITECTURE.md`, and `docs/gui_test_platform.md` say `src` owns domain state and host policy, while `vendor/radiant` owns GUI behavior and native-shell/runtime mechanics.
- Test strategy: Explicitly documented. `docs/TEST.md` and `.github/workflows/ci.yml` center the repo on `devcheck`, `ci_agent`, `ci_quick`, `ci_local`, deterministic Rust tests, and the Windows GUI contract wrappers.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/run_agent_request.ps1`, `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, `scripts/ci_local.ps1`, and `scripts/run_gui_contract.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, deterministic interaction, reversibility, and non-blocking execution.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, or retention-driven product.
- What the repo appears to be moving toward: Strongly implied by code/docs. The live tree continues to invest in host-owned GUI contracts, `radiant` native-shell/runtime correctness, and incremental cleanup that preserves strict ownership boundaries rather than broad rewrites.
- What is merely possible but unsupported: Weakly implied / uncertain. A shared action-id source across host and vendor, promotion of desktop AIV into CI, or a broad action-enum decomposition are not yet clearly mandated by current repository evidence.

## Audit Baseline

- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` is green on the current tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` is green on the current tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` is green and keeps the current score at `4`.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md`; the snapshot still records eight allowlisted oversized Rust files, including `vendor/radiant/src/app/hotkeys.rs`, three large `native_vello` test hubs, `src/app/controller/playback/transport/selection.rs`, `src/app_core/actions/catalog/kinds.rs`, and `vendor/radiant/src/app/actions/mod.rs`.
- `docs/file_size_budget_allowlist.txt` explicitly says some exceptions should stay centralized unless concrete ownership or correctness pressure emerges. That allowlist guidance is treated as repository intent during ranking.

## Ordered Backlog

### 1. [ ] Expand automation action-id parity checks from representative nodes to panel-wide contract coverage

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the repository documents the host action catalog as canonical, but the native-shell automation snapshot still advertises many action ids through manual string literals and a separate matcher. The current parity tests only sample representative nodes, so a control can still drift to a wrong-but-cataloged id without failing the default semantic contract suite.
- Evidence:
  - `docs/gui_test_platform.md` calls the host action catalog canonical.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs:98` keeps a large manual `action_slug(&UiAction)` matcher.
  - `vendor/radiant/src/gui/native_shell/state/automation/sidebar.rs:71`, `:106`, `:128`, `:223-240`, and `:289` still emit literal `available_actions` ids directly.
  - `vendor/radiant/src/gui/native_shell/state/automation/browser.rs:138`, `:214`, `:287`, `:299`, and `:337` do the same for browser and map nodes.
  - `vendor/radiant/src/gui/native_shell/state/automation/waveform.rs:140` and `:220`, plus `vendor/radiant/src/gui/native_shell/state/automation/dialogs.rs:92`, still hardcode action ids outside the `action_slug` path.
  - `src/gui_test/runner/tests/action_parity.rs` covers a representative node list rather than every manual `available_actions` emitter family.
- Recommended change: keep the current host/vendor boundary, but add broader fixture-driven parity assertions for each panel family that still hardcodes advertised action ids. Prefer panel- or node-family helpers that compare emitted ids against catalog-backed expectations over another large hand-maintained test table.
- Expected impact: reduces semantic automation drift risk for `gui-test-cli`, semantic runner assertions, and desktop AIV without forcing an ownership refactor first.
- Risks / tradeoffs: More explicit fixture assertions will make intentional action-surface changes noisier; the tests should be structured around stable node families, not brittle full-tree snapshots.
- Dependencies: none
- Suggested validation:
  - targeted `src/gui_test/runner/tests.rs` and `src/gui_test/runner/tests/action_parity.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 2. [ ] Split the `radiant` hotkey catalog into scope-owned binding slices while preserving one flat public contract

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: contextual hotkeys are a first-class interaction contract in `docs/design_principles.md`, but one allowlisted `vendor/radiant` file still owns the entire binding table, chord resolution, and route tests. That makes overloaded key edits harder to review and increases the chance of cross-scope regressions in a user-visible input surface.
- Evidence:
  - `docs/design_principles.md` says hotkeys are contextual, focus-dependent, and should remain explicit and predictable.
  - `vendor/radiant/src/app/hotkeys.rs` is still `1133` lines and allowlisted in `docs/file_size_budget_allowlist.txt`.
  - The single file owns:
    - binding declarations in `HOTKEY_BINDINGS` at `vendor/radiant/src/app/hotkeys.rs:176`
    - route resolution in `vendor/radiant/src/app/hotkeys.rs:793`
    - all tests in `vendor/radiant/src/app/hotkeys.rs:857`
  - The same file handles overloaded gestures across browser, folders, sources, waveform, and global scopes; for example plain `C`, `D`, `N`, `R`, and arrow keys are all routed by focus-sensitive cases in the bottom test block.
- Recommended change: keep `HOTKEY_BINDINGS` as the flat exported surface, but build it from smaller scope- or family-owned slices (`global`, `browser`, `folders`, `sources`, `waveform`). Add table-driven route assertions for the overloaded gesture families and chord precedence so edits stay local and reviewable.
- Expected impact: smaller diffs for hotkey changes, clearer ownership of contextual keymaps, and lower regression risk in one of the repository’s most visible interaction contracts.
- Risks / tradeoffs: Stable presentation order matters for overlays and tests, so any decomposition has to preserve ordering and exported ids exactly.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test --manifest-path vendor/radiant/Cargo.toml hotkeys -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 3. [ ] Split the oversized `native_vello` runtime gesture test hubs by interaction family

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: the largest remaining allowlisted `vendor/radiant` files are runtime interaction tests that mix many unrelated queueing and waveform-gesture cases. They are important correctness coverage, but the current shapes make failures harder to localize and invite further accumulation in already-large compatibility suites.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` reports:
    - `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs` at `1636` lines
    - `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs` at `763` lines
    - `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer/selection_creation.rs` at `713` lines
  - `docs/file_size_budget_allowlist.txt` still lists all three as explicit legacy exceptions.
  - `queue_runtime.rs` mixes wheel, scrollbar, hover clearing, autoscroll, immediate keyboard follow-up, and drag/drop queue behavior in one file.
  - `waveform_drag_finish.rs` and `waveform_pointer/selection_creation.rs` mix distinct waveform gesture families such as selection creation, shift/command modifier behavior, edit-selection drags, click-play suppression, and finish-action routing.
- Recommended change: keep the shared runtime fixtures, but split these tests into focused modules by interaction family: queue input classes, browser scrollbar/row routing, waveform selection creation, waveform finish routing, and modifier-specific gesture behavior.
- Expected impact: preserves the existing correctness coverage while making runtime regressions easier to triage and future gesture additions easier to review.
- Risks / tradeoffs: Purely structural test moves can still make targeted test filters harder to discover if module names are not chosen carefully.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 4. [ ] Revisit `playback/transport/selection.rs` now that the allowlist’s “clearer boundary” condition appears satisfied

- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters: the file-size allowlist explicitly said to revisit this module only when a clearer seam emerged than the original gesture/playback coupling. The current file now presents a visible split between gesture orchestration and playback/BPM/transient policy, which makes it a better candidate for behavior-preserving decomposition than when the note was written.
- Evidence:
  - `docs/file_size_budget_allowlist.txt` says `src/app/controller/playback/transport/selection.rs` should be revisited only if a clearer module boundary emerges.
  - The current file is still `475` lines.
  - The first half centers on gesture lifecycle entrypoints such as `start_selection_drag`, `start_edit_selection_drag`, `update_selection_drag`, `finish_selection_drag`, `clear_selection`, and their edit-selection counterparts in `src/app/controller/playback/transport/selection.rs:12-203`.
  - The second half centers on playback retargeting and selection policy helpers such as `adjust_playback_after_selection_change`, `restart_non_looped_selection_playback`, `retarget_looped_selection_playback`, `snap_to_transient`, `smart_scale_target_beats`, `apply_scaled_bpm`, and `scaled_selection_bpm` in `src/app/controller/playback/transport/selection.rs:205-371`.
  - Local unit tests already live in the same file starting at `src/app/controller/playback/transport/selection.rs:374`, so helper extraction can stay close to the existing behavior contract.
- Recommended change: keep the public controller-facing entrypoints in `selection.rs`, but extract playback-retarget helpers and snap/BPM policy into focused siblings or internal submodules so the file no longer has to carry both gesture orchestration and downstream transport policy.
- Expected impact: removes one allowlisted production exception and makes waveform selection behavior easier to reason about without changing the public controller surface.
- Risks / tradeoffs: The allowlist note is a warning that this seam used to be too coupled to split safely, so extraction should be conservative and test-led rather than driven by line count alone.
- Dependencies: none
- Suggested validation:
  - targeted tests in `src/app/controller/playback/transport/selection.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. Should stable action identifiers remain duplicated across the host catalog and the `vendor/radiant` automation layer, or is a shared declaration layer now intended?

- Evidence:
  - `docs/gui_test_platform.md` calls the host action catalog canonical.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs:98-299` still maintains a manual `action_slug` matcher.
  - `vendor/radiant/src/gui/native_shell/state/automation/{sidebar,browser,waveform,dialogs}.rs` still include literal `available_actions` ids.
- Why this matters: item 1 can safely strengthen parity tests either way, but deeper deduplication depends on whether the repository still wants a boundary-preserving duplicate representation or now prefers one declaration source.
- Affected files/modules:
  - `docs/gui_test_platform.md`
  - `src/app_core/actions/catalog/`
  - `src/gui_test/runner/tests/action_parity.rs`
  - `vendor/radiant/src/gui/native_shell/state/automation/`
- Risk if guessed incorrectly: a refactor could either violate the intended host/vendor boundary or leave a now-unwanted duplicate source in place.
- Most conservative provisional assumption: keep the host catalog canonical, preserve the current vendor-owned snapshot emission path, and expand parity tests before attempting any shared-source refactor.

### [!] 2. What evidence threshold should promote desktop AIV coverage from local-only support tooling into a default gate?

- Evidence:
  - `docs/gui_test_platform.md` says desktop AIV is still local-only and not stable enough for CI.
  - The same doc says the next step is to collect repeated local evidence before any stronger promotion.
  - `docs/TEST.md` includes the GUI contract loop in the normal quick lane but leaves AIV in a separate manual/nightly path.
- Why this matters: item 1 improves semantic automation coverage, but the repo still lacks an explicit promotion rule for when semantic contract confidence is strong enough to expand the default gate.
- Affected files/modules:
  - `docs/gui_test_platform.md`
  - `docs/TEST.md`
  - `scripts/run_gui_aiv_smoke.ps1`
  - `scripts/run_gui_aiv_suite.ps1`
- Risk if guessed incorrectly: CI could be burdened with unstable focus/foreground failures too early, or a useful regression lane could stay indefinitely non-blocking without a stated bar for promotion.
- Most conservative provisional assumption: keep desktop AIV local-only until the repo records a specific stability threshold and failure-budget policy.

### [!] 3. Does `playback/transport/selection.rs` still intentionally own both gesture orchestration and playback retarget policy, or should only the public gesture entrypoints remain there?

- Evidence:
  - `docs/file_size_budget_allowlist.txt` explicitly leaves room to revisit this file once a clearer seam exists.
  - The current code has a visible split between gesture entrypoints (`start_*`, `update_*`, `finish_*`, `clear_*`) and downstream helpers (`adjust_playback_after_selection_change`, snapping, BPM scaling).
- Why this matters: item 4 is safe only if the second cluster is now considered helper policy rather than inseparable transport behavior.
- Affected files/modules:
  - `docs/file_size_budget_allowlist.txt`
  - `src/app/controller/playback/transport/selection.rs`
- Risk if guessed incorrectly: a refactor could hide important temporal coupling between selection drags and transport/playhead state.
- Most conservative provisional assumption: keep the public entrypoints in one file and extract only clearly pure or downstream helpers first.

## Rejected Ideas

### [-] 1. Split `src/selection/range.rs` just because it sits near the file-size budget

- Why it was considered: `tmp/cleanup_audit_hotspots.md` still reports it near the top of the root-crate size hotspots.
- Why it was rejected: `docs/file_size_budget_allowlist.txt` explicitly says the file should stay together unless a genuinely separate waveform-editing subdomain appears, and the module header repeats the same cohesion argument.
- What evidence was missing: a concrete ownership or correctness seam stronger than the current “range + fades + gain” domain model.

### [-] 2. Split `vendor/radiant/src/app/actions/mod.rs` by action family

- Why it was considered: the file remains an allowlisted `781`-line action enum.
- Why it was rejected: the module docs and allowlist both explicitly say the compatibility surface should stay centralized unless stronger ownership pressure appears.
- What evidence was missing: a repository-backed decision that action-family decomposition is now preferred over one inspectable runtime/host contract surface.

### [-] 3. Treat `src/app_core/native_shell/waveform_projection.rs` and `tools/bench-cli/src/bench/gui/interactions.rs` as missing-test hotspots

- Why it was considered: `tmp/cleanup_audit_hotspots.md` flags both as likely heuristic test gaps.
- Why it was rejected: both already have dedicated neighboring test coverage in `src/app_core/native_shell/tests/waveform.rs` and `tools/bench-cli/src/bench/gui_tests.rs`.
- What evidence was missing: a real behavior gap that the existing external tests fail to cover.

### [-] 4. Reopen the historical updater-helper UI split from `tmp/cleanup_plan.md`

- Why it was considered: the parked cleanup plan still contains a historical item about splitting updater-helper UI concerns.
- Why it was rejected: the current tree already reflects that decomposition under `apps/updater-helper/src/ui/{mod,projection,state,tasks,tests}.rs`.
- What evidence was missing: any current-tree monolithic `apps/updater-helper/src/ui.rs` file or equivalent unresolved async/reducer blob.
