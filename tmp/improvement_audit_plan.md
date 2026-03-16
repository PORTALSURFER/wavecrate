# Improvement Audit Plan

Generated: 2026-03-16
Status: Phase 1 complete. Ranked backlog is refreshed for the live tree and awaits explicit user confirmation before any implementation. The four conservative open-question codifications were implemented on 2026-03-16 without starting the ranked backlog.

## Scope

- This document records a fresh evidence-driven improvement audit for the current repository state on 2026-03-16.
- Items are ranked in strict execution order by expected ROI, not by category.
- Recommendations are limited to improvements supported by live repository evidence.
- No implementation has been performed for this Phase 1 backlog.

## Repository Context

- Project purpose: Explicitly documented. `README.md` describes Sempal as a realtime-oriented sample triage and curation app for large local libraries.
- Maturity level: Explicitly documented. `README.md` labels the app early alpha and warns that bugs may modify or delete samples.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace; `vendor/radiant` owns the retained GUI/runtime stack.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` splits ownership across `src/`, `src/app_core`, `vendor/radiant`, `tools/`, and `docs/`.
- Architectural boundaries: Explicitly documented. `docs/ARCHITECTURE.md` keeps domain logic in `src`, bridge/projection logic in `app_core`, and renderer/runtime concerns in `vendor/radiant`.
- Test strategy: Explicitly documented. `docs/TEST.md`, `AGENTS.md`, and `.github/workflows/ci.yml` define the normal Windows validation path as `scripts/devcheck.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Canonical local validation commands: Explicitly documented. Windows sessions should use the PowerShell wrappers in `scripts/*.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes realtime behavior, non-blocking execution, data integrity, integrated mouse/keyboard semantics, and undoable edits.
- Explicit non-goals / unsupported promotions: Explicitly documented. `docs/gui_test_platform.md` says desktop AIV remains local-only and is not ready for CI promotion.
- Product direction beyond those documents: Strongly implied by code/docs. The live tree still favors correctness, maintainability, ownership clarity, and validation hardening over broad new end-user feature work.

## Ordered Backlog

### 1. [ ] Refresh stale cleanup-debt tracking artifacts before using them for further prioritization

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the repo’s own cleanup inputs are stale again, so future audits and cleanup passes will mis-rank already-finished work.
- Evidence:
  - `docs/file_size_budget_allowlist.txt` still allowlists historical paths that no longer exist in their old form, including `src/analysis/frequency_domain/stft.rs`, `vendor/radiant/src/app/actions.rs`, `vendor/radiant/src/gui/native_shell/layout.rs`, and `vendor/radiant/src/gui/native_shell/state/tests/chrome_layout.rs`.
  - `tmp/cleanup_audit_hotspots.md` still reports old hotspots from commit `b95a4aa2`, including already-split or renamed files such as `stft.rs`, `waveform/model.rs`, and `gui_test/packs.rs`.
  - `tmp/cleanup_plan.md` still references old cleanup targets, including the pre-split `src/app/controller/playback/audio_options.rs`.
- Recommended change: refresh the allowlist and parked hotspot snapshots, and annotate stale parked cleanup references so later audits start from the live tree rather than obsolete debt data.
- Expected impact: future cleanup prioritization stops resurfacing already-completed work and better reflects the current codebase.
- Risks / tradeoffs: this is meta-work rather than product behavior work, but the effort is small and it protects planning quality.
- Dependencies: none
- Suggested validation: rerun the cleanup/hotspot generation scripts and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 2. [ ] Decompose `handle_analysis_message(...)` into progress routing, cache invalidation, and follow-up scheduling helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: analysis progress handling is one of the main branch-heavy controller seams for long-running background work, and it still concentrates unrelated responsibilities in one function.
- Evidence:
  - `src/app/controller/library/background_jobs/analysis.rs:8` defines `handle_analysis_message(...)` as the single entrypoint for progress updates, enqueue-finished follow-up work, embedding backfill progress, and duration-update invalidation.
  - The same file handles selected-source scoping, similarity-prep routing, overlay visibility, detail-string construction, analysis snapshot updates, feature/duration cache invalidation, and follow-up message emission around `AnalysisJobMessage::EnqueueFinished` and `AnalysisJobMessage::DurationsUpdated`.
  - Direct tests now exist in the same file for several branches, which reduces test-risk but makes the remaining ownership mix more obvious rather than less important.
- Recommended change: keep the message surface stable, but split progress-overlay policy, cache invalidation/follow-up invalidation, and enqueue-finished scheduling into focused helpers or internal modules.
- Expected impact: future analysis-job changes become easier to review and less likely to regress overlay state or cache invalidation behavior.
- Risks / tradeoffs: the split must preserve selected-source gating and similarity-prep semantics exactly.
- Dependencies: none
- Suggested validation: `cargo test background_jobs::analysis -- --test-threads=1`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 3. [ ] Split `vendor/radiant/src/gui/native_shell/state/waveform_segments/mod.rs` by static-segment routing, waveform overlays, and header/image helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: waveform rendering is a user-visible hot path, and one file still mixes segment ownership decisions with several unrelated overlay and image emit families.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/waveform_segments/mod.rs:26`-`49` owns static-segment routing via `static_segment_for_primitive`, `static_segment_for_text`, and `static_segment_for_point`.
  - `vendor/radiant/src/gui/native_shell/state/waveform_segments/mod.rs:95`-`238` builds waveform playhead, selection, loop, fade, and scrollbar overlays.
  - `vendor/radiant/src/gui/native_shell/state/waveform_segments/mod.rs:240`-`295` renders the waveform loading placeholder, while `:297`-`327` emits waveform images and `:329`-`393` renders the waveform header overlay.
  - The file already has sibling modules for `fades`, `scrollbar`, `selection`, and `trail`, which strongly implies the remaining mixed helpers are ready for further decomposition.
- Recommended change: preserve the external `waveform_segments` surface, but move segment routing, loading/image emission, and header overlay assembly into focused siblings next to the existing overlay helpers.
- Expected impact: waveform-shell changes become easier to localize, and regressions in retained segment ownership are easier to diagnose separately from overlay rendering.
- Risks / tradeoffs: segment ownership logic is cache-sensitive, so the split must not change which static segment owns existing primitives or text runs.
- Dependencies: none
- Suggested validation: targeted native-shell waveform/state tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 4. [ ] Split `vendor/radiant/src/gui_runtime/native_vello/text_renderer.rs` into font loading, layout caching, atom caching, and glyph-layout helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: native text rendering affects many runtime surfaces, but font discovery, cache eviction, glyph layout, and color/icon encoding still live together in one file with no direct local test module.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/text_renderer.rs:38` defines `NativeTextRenderer` with both layout-cache and atom-cache state.
  - `vendor/radiant/src/gui_runtime/native_vello/text_renderer.rs:122`-`156` owns layout-cache lookup/eviction, while `:181`-`218` owns atom interning/eviction.
  - `vendor/radiant/src/gui_runtime/native_vello/text_renderer.rs:220`-`306` computes glyph layout and cursor stops.
  - `vendor/radiant/src/gui_runtime/native_vello/text_renderer.rs:309`-`352` handles native font discovery/loading, and the file also carries primitive color/icon conversion helpers.
  - A targeted test search found usage from runtime startup and text editing, but no focused local tests for the renderer/cache helpers themselves.
- Recommended change: keep `NativeTextRenderer` as the runtime-facing façade, but move font discovery, layout-cache policy, atom-cache policy, and pure glyph-layout computation into focused helper modules with direct tests.
- Expected impact: text-related regressions become easier to isolate, and cache-policy work stops competing with glyph-shaping and font-discovery changes in one file.
- Risks / tradeoffs: text layout is runtime-sensitive; the split should preserve cache capacity, fallback font order, and cursor-stop semantics exactly.
- Dependencies: none
- Suggested validation: targeted native-Vello text/key-binding tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 5. [ ] Separate browser path-selection cache/set logic from focus/load side effects in `src/app/controller/library/wavs/browser_actions/selection.rs`

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: browser multi-selection is now path-authoritative, but one file still mixes canonical selection-set maintenance with focus, rebuild, and load-preview side effects.
- Evidence:
  - `src/app/controller/library/wavs/browser_actions/selection.rs:8`-`116` manages canonical path/index selection cache invalidation and conversions.
  - `src/app/controller/library/wavs/browser_actions/selection.rs:118`-`204` manages anchor-based range extension and visible-row resolution.
  - `src/app/controller/library/wavs/browser_actions/selection.rs:231`-`357` mixes selection mutation with `focus_browser_context()`, `rebuild_browser_lists()`, `focus_wav_by_index_preview_with_rebuild(...)`, and `select_wav_by_index_with_rebuild(...)`.
  - There is focused surrounding behavior coverage in `src/app/controller/tests/browser_selection.rs`, `src/app/controller/tests/browser_actions/row_actions.rs`, and `src/app/controller/library/wavs/browser_actions/tests.rs`, which lowers behavior risk and highlights the remaining ownership mix.
- Recommended change: keep the public controller methods stable, but split pure selection-set/cache helpers from the action-layer methods that trigger focus, rebuild, preview-load, and marker refresh side effects.
- Expected impact: browser-selection behavior becomes easier to extend safely, and future source-of-truth changes stop requiring edits in a mixed cache/UI side-effect hub.
- Risks / tradeoffs: the split must preserve anchor semantics, visible-row behavior, and preview-vs-commit load behavior.
- Dependencies: none
- Suggested validation: targeted browser selection/action tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 6. [ ] Add direct controller coverage for audio option refresh/apply/fallback branches before refactoring the audio-options controller

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: audio host/device selection is user-visible and stateful, but the controller layer currently lacks direct tests around refresh/apply/fallback behavior.
- Evidence:
  - `src/app/controller/playback/audio_options/controller.rs:9`-`153` owns output/input refresh normalization, device probing, warning propagation, and input-channel normalization.
  - `src/app/controller/playback/audio_options/controller.rs:156`-`276` owns host/device/sample-rate setters, persistence, player rebuild, and applied-status updates.
  - `src/app/controller/playback/audio_options/controller.rs:279`-`321` formats output/input fallback warnings.
  - A targeted test search found call sites and lower-level audio enumeration modules, but no direct controller tests for `refresh_audio_options(...)`, `refresh_audio_input_options(...)`, `apply_audio_selection(...)`, or the fallback-message branches.
- Recommended change: add focused controller tests for output refresh normalization, input-channel warning normalization, successful apply/persist, rebuild failure, and fallback-warning formatting before changing structure.
- Expected impact: refactoring this controller becomes materially safer, and audio-setting regressions become easier to localize.
- Risks / tradeoffs: test harnesses may need lightweight stubs for the player rebuild path and enumerated audio backends.
- Dependencies: none, but this should precede item 7.
- Suggested validation: targeted audio-options controller tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 7. [ ] Split `src/app/controller/playback/audio_options/controller.rs` into refresh policy, apply/persist flow, and fallback-message helpers

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: audio-options control flow currently mixes pure normalization/probing policy with player rebuilds, persistence, UI projection, and user-facing fallback text.
- Evidence:
  - `src/app/controller/playback/audio_options/controller.rs` remains a 352-line controller file with both output and input branches.
  - The file interleaves pure normalization/probing work with mutation and persistence paths such as `set_audio_host(...)`, `set_audio_device(...)`, `apply_audio_selection(...)`, and `rebuild_audio_player(...)`.
  - The repo’s historical cleanup artifacts also still mention the older unsplit `audio_options.rs`, which suggests this ownership seam has been visible for multiple audit passes.
- Recommended change: after item 6 lands, move output/input refresh policy, apply/rebuild/persist flow, and fallback-message formatting into focused siblings while keeping the public controller API unchanged.
- Expected impact: audio-option logic becomes easier to navigate and future changes stop reopening one controller file for both probing policy and side effects.
- Risks / tradeoffs: audio settings are user-visible and hardware-sensitive, so the split must preserve warning text, persistence timing, and rebuild failure behavior.
- Dependencies: item 6
- Suggested validation: targeted audio-options tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 8. [ ] Split `vendor/radiant/src/gui/native_shell/state/options_panel.rs` by geometry, action definitions, rendering, and style helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: the native-shell options panel is a distinct UI surface, but one file still mixes geometry, hit-testing, action definitions, text formatting, and style-state coloring.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/options_panel.rs:12`-`46` computes status-button and right-text geometry.
  - `vendor/radiant/src/gui/native_shell/state/options_panel.rs:48`-`137` builds panel layout and point/action hit-testing.
  - `vendor/radiant/src/gui/native_shell/state/options_panel.rs:139`-`233` renders the button and panel surfaces.
  - `vendor/radiant/src/gui/native_shell/state/options_panel.rs:235`-`356` defines button/action catalogs, display text, and hover/flash style helpers.
  - Existing coverage in `vendor/radiant/src/gui/native_shell/state/tests/overlay_controls.rs` exercises some behavior, but the production ownership boundaries are still broad.
- Recommended change: preserve the current options-panel UI contract, but split geometry/hit-test helpers, button/action definitions, and rendering/style helpers into focused siblings.
- Expected impact: options-panel changes become easier to isolate and review, especially when adding or adjusting settings-specific UI.
- Risks / tradeoffs: the split should avoid duplicating style math between status-button and panel rendering.
- Dependencies: none
- Suggested validation: targeted native-shell overlay/options tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 9. [ ] Split `vendor/radiant/src/gui_runtime/native_vello/profiling.rs` into feature-gated stats aggregation, reporting, and no-op adapter helpers

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: runtime profiling is already isolated behind a feature flag, but the live implementation still combines many unrelated counters and a very large reporting/reset path in one file.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/profiling.rs:49` defines `NativeVelloProfiler` with redraw, rebuild, pull, overlay, and interaction-latency counters in one struct.
  - `vendor/radiant/src/gui_runtime/native_vello/profiling.rs:100`-`197` exposes many small counter mutators, while `:199`-`317` concentrates the full reporting, averaging, rate computation, and reset cycle inside `record_redraw(...)`.
  - The file also duplicates the public profiler surface in the non-`gui-performance` no-op implementation at `:356`-`406`.
  - A targeted search found no focused local tests for the reporting or reset behavior.
- Recommended change: keep the runtime-facing profiler API stable, but separate the feature-gated stats bucket types, reporting/formatting logic, and no-op shim into smaller helpers with direct tests where practical.
- Expected impact: runtime profiling becomes easier to evolve without accidental drift between the real and no-op implementations.
- Risks / tradeoffs: the profiler is diagnostic-only, so ROI is lower than the controller/native-shell items above; the split should avoid adding overhead in the hot path.
- Dependencies: none
- Suggested validation: targeted native-Vello runtime tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

## Open Questions / Missing Definitions

Conservative codification note:
- The current implementation now explicitly documents the conservative direction for these four questions in code comments/docs.
- They remain listed here because the architectural questions still exist, but the current preferred posture is now recorded in the relevant modules.

### 1. Should `vendor/radiant/src/app/actions/mod.rs` remain one intentionally centralized compatibility surface?

- Evidence:
  - The file is still 485 lines and remains the one `UiAction` catalog shared across runtime/native-shell boundaries.
  - Recent work already added internal family grouping without changing the top-level contract.
- Why this matters: future cleanup passes could mistake file size for accidental sprawl and destabilize a compatibility-sensitive surface.
- Affected files/modules: `vendor/radiant/src/app/actions/mod.rs`, native runtime action routing, native-shell action emission.
- Risk if guessed incorrectly: splitting the top-level enum prematurely could introduce unnecessary contract churn across runtime/native-shell bridges.
- Most conservative provisional assumption: keep one top-level `UiAction` surface unless a concrete bridge-contract mismatch appears.

### 2. Is `vendor/radiant/src/gui/layout_core/engine/context.rs` oversized but still intentionally cohesive?

- Evidence:
  - `vendor/radiant/src/gui/layout_core/engine/context.rs` is 398 lines and mixes measurement caches, virtualization caches, diagnostics, and debug primitive recording.
  - Surrounding tests in `vendor/radiant/src/gui/layout_core/engine/tests.rs`, `stress_tests.rs`, and `virtualization_tests.rs` already exercise diagnostics and virtualization behavior through the engine entrypoints.
- Why this matters: further splitting could help readability, but it could also fracture a central engine evaluation context that is intentionally shared by measure/layout/scroll passes.
- Affected files/modules: `vendor/radiant/src/gui/layout_core/engine/context.rs`, `layout.rs`, `measure.rs`, `layout/scroll*.rs`.
- Risk if guessed incorrectly: over-splitting the context could add indirection without improving correctness or testability.
- Most conservative provisional assumption: treat `LayoutContext` as acceptable until a clearer ownership or testability problem emerges.

### 3. Should `vendor/radiant/src/gui_runtime/native_vello/text_bpm.rs` remain a shared text-entry hub for browser search and waveform BPM editing?

- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/text_bpm.rs` mixes generic text-input target syncing with waveform-BPM-specific sanitization and browser-search pointer routing.
  - Surrounding runtime code already splits some related responsibilities into `text_runtime.rs`, `runtime_events/pointer.rs`, and `runtime_input/cursor.rs`.
- Why this matters: a future split could either clarify ownership or unnecessarily separate one intentionally shared single-line text-editor path.
- Affected files/modules: `vendor/radiant/src/gui_runtime/native_vello/text_bpm.rs`, `text_runtime.rs`, `runtime_events/pointer.rs`.
- Risk if guessed incorrectly: an unnecessary split could fragment one editor-state contract across multiple partially overlapping modules.
- Most conservative provisional assumption: keep one shared text-entry hub unless the browser-search and waveform-BPM paths start diverging in behavior rather than just in targets.

### 4. Is `src/selection/range.rs` large but still appropriately domain-cohesive?

- Evidence:
  - `src/selection/range.rs` is 397 lines and mixes normalized range geometry, fade parameter mutation, shift logic, and fade/gain evaluation.
  - The domain already has focused surrounding coverage in `src/selection/tests.rs`, and the types are widely reused across controller, waveform render, native bridge, and playback code.
- Why this matters: size alone suggests a split, but the file may still be the correct home for one dense but cohesive waveform-selection math surface.
- Affected files/modules: `src/selection/range.rs`, `src/selection/tests.rs`, waveform render and controller selection code.
- Risk if guessed incorrectly: splitting the core selection math too early could scatter one stable domain concept across multiple tiny helpers with little practical gain.
- Most conservative provisional assumption: keep `SelectionRange` and its core fade/gain math together unless a stronger ownership conflict appears.

## Rejected Ideas

### 1. Promote desktop AIV further into normal CI

- Why it was considered: the repo continues to invest in semantic and desktop GUI validation.
- Why it was rejected: `docs/gui_test_platform.md` still explicitly says desktop AIV remains local-only and is not ready for CI promotion.
- What evidence was missing: live evidence that foreground/focus instability is resolved well enough for normal CI use.

### 2. Reopen a broad native-shell layout split immediately

- Why it was considered: some native-shell files remain near or above the preferred size target.
- Why it was rejected: the recent layout/state cleanup already extracted several helper seams, and the remaining root surfaces still look intentionally cohesive.
- What evidence was missing: a concrete ownership conflict beyond size pressure alone.

### 3. Add another trash-controller item to this backlog

- Why it was considered: destructive flows are high-risk and were recurring audit targets in earlier passes.
- Why it was rejected: the live tree already contains direct trash controller coverage in `src/app/controller/tests/trash.rs` and split controller helpers under `src/app/controller/library/trash/`.
- What evidence was missing: a current live destructive-flow gap that is not already covered by the recent work.
