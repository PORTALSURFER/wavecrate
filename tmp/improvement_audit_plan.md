# Improvement Audit Plan

Generated: 2026-03-16
Merged backlog update: 2026-03-17
Status: Phase 2 complete. The newer remote audit refresh from `05662afb` is the merged baseline; carried-forward completed work from the earlier approved backlog is recorded below, and all merged backlog items were executed sequentially on 2026-03-17.

## Scope

- This document is the merged source of truth for the current evidence-driven improvement lane.
- It combines the refreshed remote Phase 1 audit with still-valid carried-forward work already executed locally.
- Items remain repository-specific and ranked by expected ROI for the current tree.
- Completed work and pending work are separated so future sessions do not mistake already-landed refactors for still-open backlog items.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as a realtime-oriented audio sample triage and curation tool for large local libraries.
- Maturity level: Explicitly documented. `README.md` labels the app early alpha and warns that bugs can modify or delete user data.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace; `vendor/radiant` owns the retained GUI/runtime path; Windows sessions must use `scripts/*.ps1`.
- Architectural boundaries: Explicitly documented. `docs/ARCHITECTURE.md` keeps domain/controller logic in `src`, bridge/projection logic in `src/app_core`, and GUI/runtime code in `vendor/radiant`.
- Validation path: Explicitly documented. `docs/TEST.md`, `AGENTS.md`, and CI use `scripts/devcheck.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Current priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, non-blocking work, reversibility, and data integrity.
- Current direction: Strongly implied by code/docs. The live tree continues to favor correctness, clear ownership, and validation hardening over speculative feature expansion.

## Completed Carry-Forward Work

### 1. [x] Refresh stale cleanup-debt tracking artifacts before using them for further prioritization

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it mattered: stale cleanup inputs were resurfacing already-finished work and distorting later prioritization.
- Evidence:
  - Historical allowlist and hotspot artifacts referenced files that no longer existed in their earlier form.
  - `tmp/cleanup_plan.md` still referenced pre-split cleanup targets.
- Recommended change: refresh the cleanup inputs and annotate the parked cleanup plan so later audits start from the live tree.
- Expected impact: later cleanup and audit passes stop re-ranking obsolete work.
- Risks / tradeoffs: meta-work only, but low-cost and protective of planning quality.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/prune_file_size_budget_allowlist.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Commit: `8e871ead` `docs(cleanup): refresh debt tracking inputs`
  - Assumption: the parked cleanup plan should remain historical, but the live debt-tracking inputs must stay current.
  - Validation: reran the cleanup scripts above and later passed `scripts/ci_quick.ps1`.

### 2. [x] Split `vendor/radiant/src/gui/native_shell/state/waveform_segments/mod.rs` by static-segment routing, waveform overlays, and header/image helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it mattered: one waveform shell module still mixed retained-segment routing, loading/image helpers, and overlay/header assembly.
- Evidence:
  - The module previously owned static-segment routing, waveform playhead/selection overlays, loading placeholder rendering, image emission, and header overlay assembly in one file.
  - Existing sibling modules (`fades`, `scrollbar`, `selection`, `trail`) already implied further decomposition was safe.
- Recommended change: preserve the outward `waveform_segments` API while extracting focused sibling helpers for routing, overlays, surface/image helpers, and header assembly.
- Expected impact: waveform-shell changes become easier to localize without changing retained-segment ownership behavior.
- Risks / tradeoffs: static segment assignment and overlay draw order are cache-sensitive and had to remain unchanged.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml waveform -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Commit: `vendor/radiant` `acf54336` `refactor(waveform): split native shell segment helpers`
  - Root record: `f825fe10` `refactor(waveform): record native shell segment split`
  - Assumption: retained segment ownership and waveform overlay ordering are stable contracts.
  - Validation: targeted waveform/native-shell coverage and `scripts/ci_quick.ps1` passed.

### 3. [x] Split `vendor/radiant/src/gui_runtime/native_vello/text_renderer.rs` into font loading, layout caching, atom caching, and glyph-layout helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it mattered: font discovery, cache policy, glyph layout, and text-helper utilities were still concentrated in one runtime file with no focused local tests.
- Evidence:
  - `NativeTextRenderer` previously owned layout-cache lookup, atom interning/eviction, glyph layout/cursor-stop computation, and platform font discovery in one module.
  - A targeted repository search found runtime usage but no dedicated local tests for the cache/layout helpers.
- Recommended change: keep `NativeTextRenderer` as the runtime-facing facade while extracting font, cache, and pure layout helpers, adding focused cache/layout tests.
- Expected impact: text-path changes become easier to isolate without destabilizing runtime call sites.
- Risks / tradeoffs: cache capacity, fallback font order, and cursor-stop semantics had to remain unchanged.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml text_renderer -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml key_bindings -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Commit: `vendor/radiant` `1c994f70` `refactor(text): split native vello text renderer`
  - Root record: `08e5a460` `refactor(text): record native vello renderer split`
  - Assumption: `NativeTextRenderer` remains the stable runtime-facing façade while internals move behind it.
  - Validation: targeted renderer/key-binding coverage, `scripts/run_gui_contract.ps1`, and `scripts/ci_quick.ps1` passed.

## Merged Pending Backlog

### 1. [x] Finish splitting `src/app/controller/library/background_jobs/analysis.rs` so file-size and quality-score guardrails recover

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: this remains a live guardrail violation and currently breaks the normal full-scan preflight.
- Evidence:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` currently fails on `src/app/controller/library/background_jobs/analysis.rs: 428`.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` still fails because `docs/QUALITY_SCORE.md` no longer matches the degraded guardrail state.
  - `6d42d8b8` already extracted progress-routing helpers, but the file still exceeds the 400-line hard ceiling.
- Recommended change: keep the `AnalysisJobMessage` surface stable, retain the helper split already landed in `6d42d8b8`, and move the remaining enqueue-finished, cache-invalidation, and/or local-test burden into focused siblings so the file drops under budget and the quality-score note can be corrected.
- Expected impact: restores green full-scan guardrails and reduces risk in a background-job hot path.
- Risks / tradeoffs: selected-source gating, similarity-prep routing, and progress-overlay semantics must remain unchanged.
- Dependencies: none
- Suggested validation:
  - `cargo test background_jobs::analysis -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Partial progress already landed in `6d42d8b8` `refactor(analysis): split progress message helpers`, but the item remains open because the file-size guardrail is still red.
  - Date: 2026-03-17
  - Commit: `9b3d2999` `refactor(analysis): move background job tests out of handler module`
  - Assumption: moving the local test module into a sibling file is sufficient to restore the guardrails without further handler-surface churn.
  - Validation: `cargo test background_jobs::analysis -- --test-threads=1`, `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`, `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.

### 2. [x] Decompose `src/app/controller/library/background_jobs/scan.rs` by scan completion policy, follow-up scheduling, and similarity-prep finalization

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: scan completion is still one branch-heavy controller hub for cache invalidation, status reporting, follow-up analysis work, duration backfill, and similarity-prep finalization.
- Evidence:
  - `src/app/controller/library/background_jobs/scan.rs` currently exceeds the size budget at 402 lines.
  - `handle_scan_finished(...)` remains the single completion seam for success, cancel, backfill, and failure handling.
  - The same file mixes status/cache invalidation, analysis enqueue work, duration backfill spawning, and similarity-prep finalization/cancel behavior.
- Recommended change: keep `handle_scan_finished(...)` as the public seam, but split status/cache invalidation policy, follow-up enqueue scheduling, duration backfill triggering, and similarity-prep completion/cancel logic into focused internal helpers or sibling modules.
- Expected impact: scan-completion changes become easier to review safely, and the second active file-size-budget violation is removed.
- Risks / tradeoffs: sequencing between wav reloads, cache invalidation, analysis enqueue work, and similarity-prep finalization must remain unchanged.
- Dependencies: none
- Suggested validation:
  - `cargo test background_jobs::scan -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: `9a21f8b0` `refactor(scan): split scan completion helpers`
  - Assumption: the safest decomposition is to keep `handle_scan_finished(...)` as the public seam and only extract policy/scheduling helpers around it.
  - Validation: `cargo test background_jobs::scan -- --test-threads=1`, `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.

### 3. [x] Separate browser path-selection cache maintenance from focus/load side effects in `src/app/controller/library/wavs/browser_actions/selection.rs`

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: browser multi-selection remains path-authoritative, but the controller file still mixes canonical selection maintenance with focus, rebuild, preview-load, and commit-load side effects.
- Evidence:
  - The same file owns path/index cache invalidation, anchor/range extension, visible-row resolution, focus changes, rebuild triggers, and preview/commit load flows.
  - Focused surrounding coverage already exists in `src/app/controller/tests/browser_selection.rs`, `src/app/controller/library/wavs/browser_actions/tests.rs`, and `tests/controller_browser_integration.rs`.
- Recommended change: preserve the public controller surface, but isolate pure selection-set/cache helpers from the action-layer methods that trigger focus, rebuild, marker refresh, preview-load, and commit-load side effects.
- Expected impact: browser-selection changes become safer and easier to reason about.
- Risks / tradeoffs: anchor semantics, visible-row behavior, and preview-vs-commit load behavior must remain unchanged.
- Dependencies: none
- Suggested validation:
  - `cargo test browser_selection -- --test-threads=1`
  - `cargo test browser_actions -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: `879e3ec8` `refactor(browser): split browser selection path helpers`
  - Assumption: the highest-value split is to extract the canonical path/index cache layer while keeping action-side focus and preview/commit loading in the parent controller module.
  - Validation: `cargo test browser_selection -- --test-threads=1` and `cargo test browser_actions -- --test-threads=1` passed.

### 4. [x] Add direct controller coverage for audio host/device refresh, apply, and fallback branches before refactoring audio-options control flow

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: audio settings are user-visible and hardware-sensitive, but the controller layer still lacks direct tests around refresh/apply/fallback behavior.
- Evidence:
  - `src/app/controller/playback/audio_options/controller.rs` still owns output/input refresh normalization, device probing, apply/persist behavior, and fallback warning formatting.
  - A targeted repository search found production references and call sites, but no dedicated controller tests for these branches.
- Recommended change: add focused controller tests for output refresh normalization, input-channel warning normalization, successful apply/persist, rebuild failure, and fallback-warning formatting before structural cleanup.
- Expected impact: materially lowers the risk of regressing audio configuration on real hardware and makes the follow-up refactor safer.
- Risks / tradeoffs: the test harness may need lightweight stubs around player rebuild behavior and enumerated backends.
- Dependencies: none, but this item should precede item 5.
- Suggested validation:
  - targeted audio-options controller tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: `0bb0bd1a` `refactor(audio): split audio option controller flow`
  - Assumption: the safest direct coverage is controller-module tests over refresh/apply/fallback helpers that avoid dependence on live host enumeration.
  - Validation: `cargo test audio_options -- --test-threads=1` and `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed.

### 5. [x] Split `src/app/controller/playback/audio_options/controller.rs` into refresh policy, apply/persist flow, and fallback-message helpers

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: one controller file still mixes pure normalization/probing policy with config mutation, audio-player rebuilds, UI projection, and fallback-message formatting.
- Evidence:
  - `src/app/controller/playback/audio_options/controller.rs` still combines output/input refresh policy, setter entrypoints, apply/rebuild/persist flow, and user-facing fallback text.
  - The file remains one of the visible cleanup hotspots and item 4 shows the direct controller branches are still under-tested.
- Recommended change: after item 4 lands, move output/input refresh policy, apply/rebuild/persist flow, and fallback-message formatting into focused siblings while keeping the public controller API stable.
- Expected impact: audio-settings logic becomes easier to navigate and review.
- Risks / tradeoffs: persistence timing, warning text, and rebuild failure behavior must remain unchanged.
- Dependencies: item 4
- Suggested validation:
  - targeted audio-options tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: `0bb0bd1a` `refactor(audio): split audio option controller flow`
  - Assumption: output/input refresh policy, apply/persist flow, and fallback-message formatting can move into sibling helpers without changing the public controller setters.
  - Validation: `cargo test audio_options -- --test-threads=1` and `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed.

### 6. [x] Split `vendor/radiant/src/gui/native_shell/state/options_panel.rs` by geometry, action definitions, rendering, and style helpers

- Classification: Architecture improvement
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters: the newer remote audit no longer ranked this item, but the older approved backlog’s evidence still holds and the file remains a mixed-responsibility native-shell UI surface.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/options_panel.rs` still combines geometry, hit-testing, button/action catalogs, render logic, text formatting, and hover/flash style helpers in one module.
  - Existing overlay/options coverage in `vendor/radiant/src/gui/native_shell/state/tests/overlay_controls.rs` lowers regression risk.
- Recommended change: preserve the options-panel UI contract while extracting geometry/hit-test helpers, action definitions, and rendering/style helpers into focused siblings.
- Expected impact: settings-panel changes become easier to isolate and review.
- Risks / tradeoffs: the newer remote audit deprioritized this item, so it should stay below the controller/runtime seams above unless implementation uncovers stronger evidence.
- Dependencies: none
- Suggested validation:
  - targeted native-shell overlay/options tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: `vendor/radiant` `290677c9` `refactor(runtime): split options panel and profiling helpers`
  - Assumption: the safest extraction keeps the outward options-panel layout/render/hit-test surface stable while moving action, geometry, render, and style helpers behind it.
  - Validation: `cargo test --manifest-path X:\\sempal\\vendor\\radiant\\Cargo.toml overlay_controls -- --test-threads=1` passed.

### 7. [x] Split `vendor/radiant/src/gui_runtime/native_vello/profiling.rs` into stats buckets, reporting/reset logic, and no-op adapter helpers

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: runtime profiling is still one file that mixes stats aggregation, reporting/reset behavior, and the no-op adapter surface.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/profiling.rs` still combines many counters and a large `record_redraw(...)` reporting/reset path.
  - The file also mirrors the public profiler surface in the non-`gui-performance` implementation.
  - A targeted search still found no focused local tests for reporting/reset behavior.
- Recommended change: keep the runtime-facing profiler API stable, but extract stats bucket types, reporting/reset logic, and no-op shim helpers into focused siblings with direct tests where practical.
- Expected impact: profiling changes become safer and easier to review, with less drift risk between the real and no-op implementations.
- Risks / tradeoffs: profiling is hot-path-adjacent, so the split must avoid measurable overhead.
- Dependencies: none
- Suggested validation:
  - targeted native-Vello runtime/profiling tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: `vendor/radiant` `290677c9` `refactor(runtime): split options panel and profiling helpers`
  - Assumption: the profiler API must remain available to the native-Vello runtime while feature-gated stats buckets, redraw reporting, and the no-op adapter move into sibling modules.
  - Validation: `cargo test --manifest-path X:\\sempal\\vendor\\radiant\\Cargo.toml gui_runtime::native_vello -- --test-threads=1` passed.

## Open Questions / Missing Definitions

### [!] 1. Should `vendor/radiant/src/app/actions/mod.rs` remain one intentionally centralized compatibility surface?

- Evidence:
  - The file-level docs still describe `UiAction` as an intentionally centralized compatibility surface.
  - Conservative codification for that direction was already recorded in code/docs on 2026-03-16.
- Why this matters: future cleanup passes could still mistake file size for accidental sprawl.
- Affected files/modules: `vendor/radiant/src/app/actions/mod.rs`, runtime action routing, native-shell action emission.
- Risk if guessed incorrectly: premature splitting could destabilize runtime/native-shell contract boundaries.
- Most conservative provisional assumption: keep one top-level `UiAction` surface unless a concrete bridge-contract mismatch appears.

### [!] 2. Should `src/selection/range.rs` continue to keep geometry, fades, and gain evaluation together?

- Evidence:
  - The file-level docs explicitly argue that normalized bounds, fades, and gain evaluation intentionally form one waveform-editing domain model.
  - Conservative codification for that direction was already recorded in code/docs on 2026-03-16.
- Why this matters: size pressure alone is weak evidence for splitting a dense but cohesive domain contract.
- Affected files/modules: `src/selection/range.rs`, waveform editing, selection preview, and destructive edit flows.
- Risk if guessed incorrectly: over-splitting could scatter one stable waveform-selection contract across several low-value helpers.
- Most conservative provisional assumption: keep `SelectionRange` and its fade/gain math together unless a clearer ownership conflict appears.

## Rejected Ideas

### [-] 1. Promote desktop AIV coverage into normal CI right now

- Why it was considered: the repository continues to invest in semantic GUI automation and AIV wrappers.
- Why it was rejected: `docs/gui_test_platform.md` still explicitly keeps desktop AIV local-only and not ready for CI promotion.
- What evidence was missing: repeated evidence that foreground/focus recovery is stable enough for CI.

### [-] 2. Split `vendor/radiant/src/app/actions/mod.rs` immediately

- Why it was considered: it remains a large live Rust file.
- Why it was rejected: the module docs still explicitly describe the centralized action surface as intentional compatibility structure.
- What evidence was missing: a concrete runtime/host-bridge contract problem that a split would solve better than internal helper organization.

### [-] 3. Split `src/selection/range.rs` immediately

- Why it was considered: the file remains dense and near the preferred size ceiling.
- Why it was rejected: the file-level docs make an explicit cohesion argument, and the current tree still lacks a stronger ownership or correctness problem than size pressure alone.
- What evidence was missing: a recurring maintenance or testability problem that clearly justifies separating selection geometry from fade/gain evaluation.
