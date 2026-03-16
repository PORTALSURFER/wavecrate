# Improvement Audit Plan

Generated: 2026-03-16
Status: Phase 2 in progress. Ranked backlog items 1-8 are approved for execution, and the user has also approved the conservative follow-ups for the four open questions.

## Scope

- This document records a fresh evidence-driven improvement audit for the live repository state on 2026-03-16.
- Items are ranked in strict execution order by expected ROI, not by category.
- Implementation is proceeding in ranked order for items 1-8.
- The four open-question follow-ups are out-of-band scope added after Phase 1 confirmation and will be executed after item 8 unless an earlier item naturally overlaps the same file.

## Phase 2 Order Notes

- The user explicitly confirmed implementation of items 1-8 on 2026-03-16.
- After Phase 1, the user also approved the conservative recommendations for the open questions around `vendor/radiant/src/app/actions.rs`, `vendor/radiant/src/gui/native_shell/layout.rs`, `vendor/radiant/src/gui/native_shell/state.rs`, and `src/app/controller/ui/hotkeys_controller/waveform.rs`.
- To preserve the ranked backlog order, those four follow-ups are treated as a second execution block after item 8 unless a ranked item safely overlaps the same file.

## Repository Context

- Project purpose: Explicitly documented. [README.md](/X:/sempal/README.md) describes Sempal as a realtime-oriented sample triage and curation app for large local libraries.
- Maturity level: Explicitly documented. [README.md](/X:/sempal/README.md) labels the app early alpha and warns that bugs may modify or delete samples.
- Primary languages / frameworks / tooling: Explicitly documented. [Cargo.toml](/X:/sempal/Cargo.toml) defines a Rust 2024 workspace; [docs/ARCHITECTURE.md](/X:/sempal/docs/ARCHITECTURE.md) and [README.md](/X:/sempal/README.md) place the retained GUI/runtime stack in `vendor/radiant`.
- Repository shape: Explicitly documented. [README.md](/X:/sempal/README.md) and [docs/ARCHITECTURE.md](/X:/sempal/docs/ARCHITECTURE.md) split ownership across `src/`, `src/app_core`, `vendor/radiant`, `tools/`, and `docs/`.
- Architectural boundaries: Explicitly documented. [docs/ARCHITECTURE.md](/X:/sempal/docs/ARCHITECTURE.md) presents `app_core` as the bridge layer and keeps renderer/runtime concerns in `vendor/radiant`.
- Test strategy: Explicitly documented. [docs/TEST.md](/X:/sempal/docs/TEST.md), [AGENTS.md](/X:/sempal/AGENTS.md), and [.github/workflows/ci.yml](/X:/sempal/.github/workflows/ci.yml) define the normal Windows validation path as `scripts/devcheck.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Canonical local validation commands: Explicitly documented. [docs/TEST.md](/X:/sempal/docs/TEST.md) and [AGENTS.md](/X:/sempal/AGENTS.md) point to the PowerShell wrappers as the default Windows flow.
- Documented priorities: Explicitly documented. [docs/design_principles.md](/X:/sempal/docs/design_principles.md) prioritizes realtime behavior, non-blocking execution, data integrity, integrated mouse/keyboard semantics, and undoable edits.
- Explicit non-goals / unsupported promotions: Explicitly documented. [docs/gui_test_platform.md](/X:/sempal/docs/gui_test_platform.md) says desktop AIV remains local-only and is not ready for CI promotion.
- Product direction beyond those documents: Weakly implied / uncertain. The current repository evidence strongly favors correctness, maintainability, and tooling hardening over broad new end-user feature work.

## Ordered Backlog

### 1. [x] Refresh stale cleanup-debt tracking artifacts before using them for further prioritization

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the repo’s own cleanup inputs are stale again, so future audits and cleanup passes will mis-rank already-finished work.
- Evidence:
  - [docs/file_size_budget_allowlist.txt](/X:/sempal/docs/file_size_budget_allowlist.txt) still allowlists already-split files such as `vendor/radiant/src/gui/native_shell/layout_adapter/controls.rs`, `vendor/radiant/src/gui/native_shell/layout_adapter/overlays/text.rs`, `vendor/radiant/src/gui/native_shell/mod.rs`, `vendor/radiant/src/gui_runtime/native_vello/tests/browser_pointer.rs`, and `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer.rs`.
  - [tmp/cleanup_audit_hotspots.md](/X:/sempal/tmp/cleanup_audit_hotspots.md) still reports previously split or removed hotspots.
  - [tmp/cleanup_plan.md](/X:/sempal/tmp/cleanup_plan.md) still references removed historical paths.
- Recommended change: refresh the file-size allowlist and hotspot snapshots, and annotate stale parked cleanup entries so later audits start from the live tree rather than obsolete debt data.
- Expected impact: future cleanup prioritization stops resurfacing already-completed work and better reflects the live codebase.
- Risks / tradeoffs: this is meta-work rather than product behavior work, but the effort is small and it protects planning quality.
- Dependencies: none
- Suggested validation: rerun the cleanup/hotspot generation scripts and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Commit: `a9e48221` `docs(audit): refresh cleanup debt inputs`
  - Assumption: the parked cleanup plan should stay historical, but its maintenance note must still describe the live debt-tracking state accurately enough that future audits do not reuse obsolete path references.
  - Validation: reran `powershell -ExecutionPolicy Bypass -File scripts/prune_file_size_budget_allowlist.ps1`, reran `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 2. [x] Add direct controller coverage for `handle_scan_finished(...)` success, backfill, cancel, and failure branches

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: scan completion drives cache invalidation, status updates, wav reloads, similarity-prep handoff, analysis enqueue, embedding backfill, and duration refresh, but the branchy completion handler is still only exercised indirectly.
- Evidence:
  - [src/app/controller/library/background_jobs/scan.rs](/X:/sempal/src/app/controller/library/background_jobs/scan.rs) centralizes almost all scan completion behavior in `handle_scan_finished(...)`.
  - The handler mixes selected-source checks, auto-scan suppression, cache invalidation, `queue_wav_load()`, similarity-prep completion, `enqueue_jobs_for_source(...)`, `enqueue_jobs_for_source_backfill(...)`, `enqueue_jobs_for_embedding_backfill(...)`, and `update_missing_durations_for_source(...)`.
  - A targeted search found references in polling/message-router code and generic polling tests, but no direct local tests for `handle_scan_finished(...)` branch behavior.
- Recommended change: add focused controller tests for changed-sample success, unchanged-sample backfill, similarity-prep short-circuit, cancel, and error branches before any structural refactor.
- Expected impact: one of the more side-effect-heavy library handlers becomes safer to change and regressions become easier to localize.
- Risks / tradeoffs: tests need thread/message harness setup and may require lightweight stubs for spawned job signaling.
- Dependencies: none, but this should precede any structural split of scan completion logic.
- Suggested validation: targeted scan/background-job tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Commit: `bf492243` `test(scan): cover scan completion branches`
  - Assumption: queue-wav-load side effects are part of the selected-source success path, so the most stable direct assertions are on cache invalidation, follow-up job enqueue, and similarity-prep transitions rather than the transient status text.
  - Validation: ran `cargo test background_jobs::scan -- --test-threads=1` and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 3. [x] Decompose `src/analysis/frequency_domain/stft.rs` into frame-loop orchestration, spectral-stat helpers, and power/SIMD helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: this is a correctness-sensitive analysis path and it still mixes data types, frame orchestration, DSP helpers, spectral statistics, band-energy computation, and SSE3-specific code in one over-budget module.
- Evidence:
  - [src/analysis/frequency_domain/stft.rs](/X:/sempal/src/analysis/frequency_domain/stft.rs) is 431 lines.
  - The file defines `FrameSet`, `SpectralFrame`, `BandFrame`, `StftProcessor`, `compute_frames(...)`, `process_frame(...)`, `spectral_from_power(...)`, `bands_from_power(...)`, and `power_spectrum_sse3_into(...)`.
  - The local test module covers only three `compute_frames(...)` cases and does not directly isolate spectral-stat or SIMD-helper behavior.
- Recommended change: split the STFT entrypoint from spectral-stat aggregation and power-spectrum helpers, keeping the public contract unchanged while creating direct unit-test seams for the pure DSP pieces.
- Expected impact: analysis correctness work becomes easier to review and extend; SIMD and stat logic can be tested independently from the frame loop.
- Risks / tradeoffs: DSP code is brittle, so boundaries should stay local and behavior-preserving rather than introducing new abstractions.
- Dependencies: none
- Suggested validation: targeted STFT/frequency-domain tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Commit: `3a84f819` `refactor(analysis): split stft helpers`
  - Assumption: the safest split keeps the STFT entrypoint unchanged and only separates frame types, power-spectrum helpers, and spectral-stat derivation into sibling helpers under one `stft` module.
  - Validation: ran `cargo test frequency_domain:: -- --test-threads=1`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 4. [x] Separate cache fingerprints, state-diff planning, and signature hashing in `vendor/radiant/src/gui_runtime/native_vello/scene_cache.rs`

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: native Vello rebuild behavior depends on stable cache fingerprints and diff planning, but several unrelated cache contracts still share one file.
- Evidence:
  - [vendor/radiant/src/gui_runtime/native_vello/scene_cache.rs](/X:/sempal/vendor/radiant/src/gui_runtime/native_vello/scene_cache.rs) is 368 lines.
  - The file combines image-upload cache keys, overlay fingerprints, static-segment fingerprints, `StaticSegmentStateGraph`, `StaticSegmentDiffPlan`, scene-cache entries, and low-level fingerprint mixing helpers.
  - The signatures span both shell/model state and static segment rebuild decisions, which obscures ownership when regressions appear in retained rendering.
- Recommended change: split cache-key/fingerprint types, static diff planning, and generic signature-mixing helpers into focused modules while preserving the runtime cache contract.
- Expected impact: retained-render cache bugs become easier to localize, and future cache-key changes stop competing inside one mixed file.
- Risks / tradeoffs: cache code is sensitive to behavior drift, so the split should avoid changing hash content or rebuild decisions.
- Dependencies: none
- Suggested validation: targeted native-Vello cache/runtime tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Commit: `vendor/radiant ed041837` `refactor(runtime): split scene cache helpers`
  - Assumption: the safest split preserves the existing cache-key, signature, and retained-scene names at the `native_vello` module boundary while moving their implementations into focused `scene_cache/*` helpers.
  - Validation: ran `cargo test --manifest-path X:\\sempal\\vendor\\radiant\\Cargo.toml gui_runtime::native_vello -- --test-threads=1` and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 5. [x] Separate `src/waveform/model.rs` public waveform types from peak-span sampling and renderer façade helpers

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: one public waveform model file still owns multiple unrelated concerns: public data types, peak-span queries, view sampling, cache-token plumbing, and renderer façade state.
- Evidence:
  - [src/waveform/model.rs](/X:/sempal/src/waveform/model.rs) is 399 lines.
  - The file defines `WaveformRgba`, `WaveformImage`, `LoadedWaveform`, `DecodedWaveform`, `WaveformPeaks`, `WaveformChannelView`, `WaveformColumnView`, `WaveformRenderer`, and helper functions like `next_cache_token()`, `max_abs_from_samples(...)`, and `max_abs_from_peaks(...)`.
  - Downstream code imports many of these types independently, while other waveform responsibilities already live in split modules such as `waveform/render/*`, `waveform/decode/*`, and `waveform/zoom_cache/*`.
- Recommended change: keep the public API stable but move peak-span math, view-sampling helpers, and renderer-facade internals beside their owning modules so `model.rs` becomes a true data-model surface.
- Expected impact: the waveform subsystem becomes easier to navigate, and public API types stop sharing a file with lower-level helper logic.
- Risks / tradeoffs: the split must preserve re-export behavior and documentation for downstream callers.
- Dependencies: none
- Suggested validation: targeted waveform render/decode/cache tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Commit: `d2efb0a5` `refactor(waveform): split model helpers`
  - Assumption: the public `crate::waveform::*` surface should remain unchanged while the implementation moves into `model/*` helpers for types, peak-span math, and renderer state.
  - Validation: ran `cargo test waveform::model -- --test-threads=1`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 6. [x] Split `src/gui_test/packs.rs` into pack registry and fixture-family scenario builders

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: the repo explicitly leans on named scenario packs for semantic GUI validation, but the live contract-smoke pack still centralizes many unrelated scenario builders in one file.
- Evidence:
  - [src/gui_test/packs.rs](/X:/sempal/src/gui_test/packs.rs) is 382 lines.
  - The file contains `gui_scenario_pack(...)`, `contract_smoke_pack()`, and nine scenario builders spanning browser, transport, waveform, map, options, prompt, and update-panel flows.
  - [docs/gui_test_platform.md](/X:/sempal/docs/gui_test_platform.md) documents named packs as a first-class validation surface and explicitly calls out the `contract-smoke` pack.
- Recommended change: keep the pack name and scenario contracts stable, but move scenario definitions into smaller modules grouped by fixture family or interaction surface.
- Expected impact: GUI scenario maintenance becomes easier, and new contract slices can be added without reopening one omnibus pack file.
- Risks / tradeoffs: over-modularizing the pack registry would hurt discoverability, so the registry should remain shallow and explicit.
- Dependencies: none
- Suggested validation: targeted GUI pack tests plus `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-16
  - Commit: pending
  - Assumption: the `contract-smoke` pack name, scenario order, and scenario bodies should remain stable while only the scenario-builder ownership moves into fixture-oriented modules.
  - Validation: ran `cargo test gui_test::packs -- --test-threads=1`, `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 7. [ ] Break the native-shell chrome-layout regression hub into ownership-aligned test modules

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: the remaining over-budget native-shell layout regression suite mixes unrelated contract families, which makes targeted maintenance slower and obscures what broke when layout contracts move.
- Evidence:
  - [vendor/radiant/src/gui/native_shell/state/tests/chrome_layout.rs](/X:/sempal/vendor/radiant/src/gui/native_shell/state/tests/chrome_layout.rs) is 404 lines.
  - The file mixes browser action-model signatures, waveform toolbar flags, sidebar band layout, scrollbar-lane geometry, seam borders, chrome motion overlay behavior, and toolbar hit-cell invariants.
  - The production layout code around [vendor/radiant/src/gui/native_shell/layout.rs](/X:/sempal/vendor/radiant/src/gui/native_shell/layout.rs) and surrounding state helpers is already decomposed enough that the regression suite is now one of the larger ownership hubs.
- Recommended change: split the layout regression hub by behavior family such as sidebar bands, waveform toolbar contracts, chrome seams, and motion overlays, while keeping the current assertions and fixtures intact.
- Expected impact: layout regressions become easier to triage and modify, and test ownership better matches the production surfaces they protect.
- Risks / tradeoffs: test-file splits can add navigation overhead if taken too far, so the new modules should follow real contract families rather than tiny per-test fragments.
- Dependencies: none
- Suggested validation: targeted native-shell layout tests and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

### 8. [ ] Split `tools/analysis-admin/src/bin/sempal-hdbscan.rs` into argument parsing, embedding load/validation, clustering policy, and writeback helpers

- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters: the developer clustering CLI is one of the larger support-tool files and still mixes parsing, DB I/O, clustering setup, noise-policy enforcement, and writeback in one executable module.
- Evidence:
  - [tools/analysis-admin/src/bin/sempal-hdbscan.rs](/X:/sempal/tools/analysis-admin/src/bin/sempal-hdbscan.rs) is 399 lines.
  - The file contains `parse_args(...)`, `load_embeddings(...)`, clustering hyper-parameter setup, label summarization, noise-policy handling, and `write_clusters(...)` in one module.
  - The local tests focus on argument parsing and noise-policy behavior, leaving the file’s ownership boundaries broader than similar split support code elsewhere in `tools/`.
- Recommended change: preserve CLI behavior but separate parse/validate, embedding load, clustering summarization, and writeback concerns into smaller helpers or internal modules.
- Expected impact: maintenance of analysis-admin tooling gets easier and future clustering-policy changes become less error-prone.
- Risks / tradeoffs: this is support-tooling work, so ROI is lower than the main runtime/controller items above.
- Dependencies: none
- Suggested validation: targeted `analysis-admin` tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Product clarification required: No

## Open Questions / Missing Definitions

### 1. Should `vendor/radiant/src/app/actions.rs` remain one intentionally centralized compatibility surface?

- Evidence:
  - [vendor/radiant/src/app/actions.rs](/X:/sempal/vendor/radiant/src/app/actions.rs) is 450 lines.
  - The file is a single documented `UiAction` enum spanning browser, transport, folder, prompt, waveform, and options actions.
  - It appears to be the shared runtime/native-shell compatibility contract rather than an accidental omnibus file.
- Why this matters: splitting an intentional compatibility surface could reduce discoverability and destabilize one of the repo’s core bridge contracts.
- Affected files/modules: [vendor/radiant/src/app/actions.rs](/X:/sempal/vendor/radiant/src/app/actions.rs), [src/app_core/actions/catalog/entries.rs](/X:/sempal/src/app_core/actions/catalog/entries.rs)
- Risk if guessed incorrectly: an unnecessary split could make action evolution harder and blur the single source of truth for emitted UI intents.
- Most conservative provisional assumption: keep `UiAction` centralized unless a concrete contradiction appears in the runtime/native-shell contract.

### 2. Is `vendor/radiant/src/gui/native_shell/layout.rs` oversized but still intentionally cohesive?

- Evidence:
  - [vendor/radiant/src/gui/native_shell/layout.rs](/X:/sempal/vendor/radiant/src/gui/native_shell/layout.rs) is 403 lines.
  - The file owns retained shell layout geometry, hit testing, and layout-contract snapshots for the same conceptual surface.
  - Current responsibilities are broad, but they are still tightly coupled to shell geometry construction.
- Why this matters: it is a candidate for future cleanup, but the evidence does not yet clearly show a safe ownership split rather than legitimate cohesion.
- Affected files/modules: [vendor/radiant/src/gui/native_shell/layout.rs](/X:/sempal/vendor/radiant/src/gui/native_shell/layout.rs)
- Risk if guessed incorrectly: a premature split could create artificial boundaries in the shell geometry model and make layout reasoning harder.
- Most conservative provisional assumption: defer backloging this until the file grows further or a specific mixed-responsibility pain point is documented.

### 3. Does `vendor/radiant/src/gui/native_shell/state.rs` still need further decomposition, or is it now functioning as a thin façade?

- Evidence:
  - [vendor/radiant/src/gui/native_shell/state.rs](/X:/sempal/vendor/radiant/src/gui/native_shell/state.rs) is 398 lines.
  - The file imports many already-split submodules and appears to act as a façade over frame build, hit testing, toolbar helpers, overlays, automation, and cache state.
- Why this matters: another split may or may not improve clarity; the file may already be at the intended orchestration boundary.
- Affected files/modules: [vendor/radiant/src/gui/native_shell/state.rs](/X:/sempal/vendor/radiant/src/gui/native_shell/state.rs)
- Risk if guessed incorrectly: unnecessary façade fragmentation could make cross-state navigation worse instead of better.
- Most conservative provisional assumption: treat `state.rs` as acceptable until a concrete ownership collision or testability issue appears.

### 4. Should `src/app/controller/ui/hotkeys_controller/waveform.rs` stay as a single waveform command hub?

- Evidence:
  - [src/app/controller/ui/hotkeys_controller/waveform.rs](/X:/sempal/src/app/controller/ui/hotkeys_controller/waveform.rs) is 377 lines.
  - The file mixes hotkey dispatch, BPM/transient toggles, deletion/navigation logic, and selection normalization helpers.
  - There is some local/direct coverage, but the file still spans multiple controller concerns.
- Why this matters: it could be a future cleanup candidate, but current evidence is weaker than the ranked items above because some command-hub centralization may be intentional.
- Affected files/modules: [src/app/controller/ui/hotkeys_controller/waveform.rs](/X:/sempal/src/app/controller/ui/hotkeys_controller/waveform.rs)
- Risk if guessed incorrectly: splitting it prematurely could scatter hotkey flow ownership and make command routing harder to follow.
- Most conservative provisional assumption: keep it centralized until there is stronger evidence of testability or ownership pain.

## Rejected Ideas

### 1. Promote desktop AIV work further into normal CI

- Why it was considered: GUI docs and recent work emphasize semantic and desktop automation.
- Why it was rejected: [docs/gui_test_platform.md](/X:/sempal/docs/gui_test_platform.md) still explicitly says desktop AIV is local-only and not stable enough for CI promotion.
- What evidence was missing: repo evidence that foreground/focus instability is resolved well enough for normal CI promotion.

### 2. Add another trash-controller backlog item

- Why it was considered: destructive flows are high-risk and had been a recurring audit target.
- Why it was rejected: the live tree now already contains direct trash controller coverage in [src/app/controller/tests/trash.rs](/X:/sempal/src/app/controller/tests/trash.rs) and the controller itself has already been split into focused submodules under [src/app/controller/library/trash](/X:/sempal/src/app/controller/library/trash).
- What evidence was missing: a current live gap beyond the already-covered rollback and destructive-flow branches.

### 3. Split `vendor/radiant/src/app/actions.rs` immediately because it is over 400 lines

- Why it was considered: the file is over the preferred size target.
- Why it was rejected: current evidence still points to intentional compatibility-surface centralization rather than accidental mixed ownership.
- What evidence was missing: a concrete mismatch between the current single-enum shape and the documented runtime/native-shell bridge contract.
