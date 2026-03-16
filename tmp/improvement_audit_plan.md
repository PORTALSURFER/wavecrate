# Improvement Audit Plan

Generated: 2026-03-16
Status: Phase 1 audit complete; awaiting explicit approval for Phase 2 sequential implementation.

## Scope

- This document records a fresh evidence-driven improvement audit for the live repository state on 2026-03-16.
- Items are ranked in strict execution order by expected ROI, not by category.
- Recommendations are limited to improvements supported by current repository evidence.
- This refresh supersedes the earlier stale snapshot in this file that incorrectly claimed Phase 2 was already in progress.

## Baseline Notes

- Current agent preflight is degraded:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` fails on `src/app/controller/library/background_jobs/analysis.rs: 428`.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` then fails because `docs/QUALITY_SCORE.md:24` still reports `Agent-facing guardrails` as healthy while the file-size guardrail is degraded.
- The handoff files outside this plan (`AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`) already describe a Phase 1 audit awaiting confirmation, so this rewritten plan is now the source of truth for that state.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as a realtime-oriented audio sample triage and curation tool for sustained listening, rapid navigation, and trustworthy destructive editing.
- Maturity level: Explicitly documented. `README.md` labels the app early alpha and warns that bugs can modify or delete user library data.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace; `vendor/radiant` owns the retained GUI/runtime path; PowerShell wrappers under `scripts/*.ps1` are the canonical Windows workflow.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` splits responsibilities across `src/` domain/controller code, `src/app_core` projection/bridge logic, `vendor/radiant` GUI/runtime code, workspace apps, tools, and docs.
- Architectural boundaries: Explicitly documented. `docs/ARCHITECTURE.md` and the migration-boundary scripts keep domain logic in `src`, migration/projection logic in `src/app_core`, and GUI behavior/runtime concerns in `vendor/radiant`.
- Test strategy: Explicitly documented. `docs/TEST.md` and `.github/workflows/ci.yml` use `cargo fmt`, `cargo clippy`, `cargo nextest`, and `cargo test --doc`, with Windows wrappers `scripts/devcheck.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Canonical local validation commands: Explicitly documented. `AGENTS.md`, `README.md`, and `docs/README.md` all direct Windows sessions to the PowerShell wrappers in `scripts/*.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes realtime responsiveness, non-blocking execution, predictable interaction semantics, reversibility, and data integrity.
- Explicit non-goals / validation constraints: Explicitly documented. `docs/gui_test_platform.md` keeps desktop AIV local-only and explicitly says it is not yet stable enough to promote into CI.
- Current improvement direction: Strongly implied by code/docs. Recent plans and guardrails prioritize correctness, ownership clarity, focused refactors, and validation hardening over speculative new product work.

## Ordered Backlog

### 1. [ ] Split `src/app/controller/library/background_jobs/analysis.rs` so the file-size budget and agent preflight recover

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: this is the only live full-scan file-size-budget violation, and it currently causes the normal agent preflight and quality-score drift check to fail before any implementation work starts.
- Evidence:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` currently fails on `src/app/controller/library/background_jobs/analysis.rs: 428`.
  - `src/app/controller/library/background_jobs/analysis.rs:8` dispatches all `AnalysisJobMessage` variants from one entrypoint.
  - `src/app/controller/library/background_jobs/analysis.rs:34` owns progress scoping, selected-source gating, overlay lifecycle, and similarity-prep routing.
  - `src/app/controller/library/background_jobs/analysis.rs:234` and `:271` also handle enqueue follow-up scheduling and browser-analysis cache invalidation in the same module.
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` fails immediately afterward because `docs/QUALITY_SCORE.md:24` still describes agent-facing guardrails as healthy.
- Recommended change: keep the `AnalysisJobMessage` surface stable, but extract focused helpers/modules for progress routing, enqueue-finished follow-up scheduling, and cache invalidation so the file falls back under the 400-line budget.
- Expected impact: restores a green full-scan file-size guardrail, reduces review risk in a background-job hot path, and removes the current preflight/score-drift mismatch without changing user-facing behavior.
- Risks / tradeoffs: selected-source gating, similarity-prep progress routing, and progress-overlay semantics are easy to regress if the split changes control-flow order.
- Dependencies: none
- Suggested validation:
  - `cargo test background_jobs::analysis -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 2. [ ] Decompose `src/app/controller/library/background_jobs/scan.rs` by scan completion policy, follow-up job scheduling, and similarity-prep finalization

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: scan completion is a long-running background-work boundary that currently mixes cache invalidation, status reporting, follow-up enqueue work, duration backfill spawning, and similarity-prep lifecycle decisions in one controller function.
- Evidence:
  - `src/app/controller/library/background_jobs/scan.rs:21` defines `handle_scan_finished(...)` as a single completion hub for every scan outcome.
  - The same function branches into follow-up analysis enqueue work at `:80` and `:105`, duration backfill work at `:141`, similarity-prep completion at `:100` and `:161`, and cancel/error handling at `:167` and `:173`.
  - `docs/design_principles.md` explicitly prioritizes non-blocking execution and observable long-running work, making this controller boundary a high-value seam for clearer ownership.
  - Local tests exist in the same file, which lowers behavioral risk and makes the remaining responsibility mix more obvious rather than less important.
- Recommended change: keep `handle_scan_finished(...)` as the public seam, but move status/cache invalidation policy, analysis/duration follow-up scheduling, and similarity-prep completion/cancel logic into focused internal helpers or sibling modules.
- Expected impact: background-scan changes become easier to review safely, and follow-up analysis behavior becomes less likely to regress when scan handling changes.
- Risks / tradeoffs: sequencing matters here; the split must preserve current ordering between cache invalidation, wav reloads, analysis enqueue work, and similarity-prep finalization.
- Dependencies: item 1 is the current guardrail recovery priority, but this item does not depend on it technically.
- Suggested validation:
  - `cargo test background_jobs::scan -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 3. [ ] Separate browser path-selection cache maintenance from focus/load side effects in `src/app/controller/library/wavs/browser_actions/selection.rs`

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: browser multi-selection is user-visible and stateful, but the current implementation still mixes canonical selection-set maintenance with focus changes, rebuild triggers, and waveform/audio load intent.
- Evidence:
  - `src/app/controller/library/wavs/browser_actions/selection.rs:8`, `:24`, `:36`, `:77`, and `:109` manage canonical selected-path state, selected-index cache invalidation, and conversions between paths and absolute indices.
  - `src/app/controller/library/wavs/browser_actions/selection.rs:172` and `:310` then combine selection mutation with `focus_browser_context()`, `rebuild_browser_lists()`, `focus_wav_by_index_preview_with_rebuild(...)`, `select_wav_by_index_with_rebuild(...)`, and marker refreshes.
  - Existing focused coverage already exists in `src/app/controller/tests/browser_selection.rs`, `src/app/controller/library/wavs/browser_actions/tests.rs`, and `tests/controller_browser_integration.rs`.
- Recommended change: preserve the public controller API, but isolate pure selection-set/cache helpers from action-layer code that triggers focus, rebuild, preview-load, and commit-load side effects.
- Expected impact: future browser selection changes become easier to reason about, and cached-path/index maintenance can evolve without reopening side-effect-heavy controller paths.
- Risks / tradeoffs: anchor semantics, preview-vs-commit load behavior, and visible-row mapping must remain unchanged.
- Dependencies: none
- Suggested validation:
  - `cargo test browser_selection -- --test-threads=1`
  - `cargo test browser_actions -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 4. [ ] Add direct controller coverage for audio host/device refresh, apply, and fallback branches before refactoring audio-options control flow

- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: audio settings are user-visible, hardware-sensitive, and stateful, but the controller layer currently has no dedicated tests around refresh/apply/fallback behavior.
- Evidence:
  - `src/app/controller/playback/audio_options/controller.rs:9` and `:63` own output/input refresh normalization, device probing, sample-rate handling, and input-channel normalization.
  - `src/app/controller/playback/audio_options/controller.rs:156-238` owns host/device/sample-rate setters plus apply/persist behavior.
  - `src/app/controller/playback/audio_options/controller.rs:279` and `:315` format user-facing output/input fallback warnings.
  - A targeted repository search for `refresh_audio_options`, `refresh_audio_input_options`, `apply_audio_selection`, `audio_fallback_message`, and `audio_input_fallback_message` only returned production references and controller call sites, with no dedicated tests under `src/` or `tests/`.
- Recommended change: add focused controller tests for output refresh normalization, input-channel warning normalization, successful apply/persist, rebuild failure, and fallback-warning formatting before structural cleanup.
- Expected impact: lowers the risk of regressing audio configuration on real hardware and creates a safer foundation for the follow-up refactor in item 5.
- Risks / tradeoffs: the test harness will likely need lightweight stubs or helper seams around player rebuild behavior and enumerated audio backends.
- Dependencies: none, but this item should precede item 5.
- Suggested validation:
  - targeted audio-options controller tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 5. [ ] Split `src/app/controller/playback/audio_options/controller.rs` into refresh policy, apply/persist flow, and fallback-message helpers

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: one controller file still mixes pure normalization/probing policy with config mutation, audio-player rebuilds, UI projection updates, and user-facing fallback text.
- Evidence:
  - `src/app/controller/playback/audio_options/controller.rs:9`, `:63`, `:156`, `:238`, `:263`, `:279`, and `:315` show refresh policy, setter entrypoints, apply/persist flow, player rebuild, and fallback-string formatting all living together.
  - The file is one of the cleanup hotspot heuristics called out by `tmp/cleanup_audit_hotspots.md`, and item 4 shows the direct controller branches are still under-tested.
- Recommended change: after item 4 lands, move output/input refresh policy, apply/rebuild/persist behavior, and fallback-message formatting into focused siblings while keeping the public controller methods stable.
- Expected impact: audio-settings changes become easier to review and test, with less coupling between hardware probing and UI/persistence side effects.
- Risks / tradeoffs: audio settings are platform-sensitive; the split must preserve persistence timing, warning text, and rebuild failure behavior exactly.
- Dependencies: item 4
- Suggested validation:
  - targeted audio-options tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 6. [ ] Split `vendor/radiant/src/gui_runtime/native_vello/text_renderer.rs` into font discovery, layout-cache policy, atom-cache policy, and glyph-layout helpers with focused tests

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: text rendering affects many runtime surfaces, but font loading, layout caching, atom interning, glyph-layout computation, and primitive conversion helpers still live in one file with no focused local tests.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/text_renderer.rs:38` defines `NativeTextRenderer` with both layout-cache and atom-cache state.
  - `:122`, `:161`, `:181`, `:203`, and `:220` mix layout lookup, cache-counter reset, atom interning/eviction, and glyph-layout computation.
  - `:309` and `:319` own platform font discovery/loading, while `:381` and `:385` also keep color/icon conversion helpers in the same file.
  - A targeted repository search for `NativeTextRenderer`, `layout_for`, `intern_text`, `compute_layout`, `load_native_font`, and `native_font_candidates` only returned production/runtime references, with no focused test module hits.
- Recommended change: keep `NativeTextRenderer` as the runtime-facing facade, but extract font discovery, cache policy, and pure glyph-layout work into smaller helpers and add direct tests for the non-runtime-sensitive pieces.
- Expected impact: text-path changes become easier to localize, and cache-policy work stops sharing one file with font-discovery and glyph-layout behavior.
- Risks / tradeoffs: text layout is visually sensitive; the split must preserve cache capacity, fallback font order, and cursor-stop semantics exactly.
- Dependencies: none
- Suggested validation:
  - targeted native-Vello text tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 7. [ ] Split `vendor/radiant/src/gui_runtime/native_vello/profiling.rs` into stats buckets, reporting/reset logic, and no-op shim helpers with direct tests where practical

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: runtime profiling is feature-gated and diagnostic-only, but the live implementation still combines counter storage, interaction-latency aggregation, large reporting/reset logic, and the no-op fallback surface in one file.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/profiling.rs:10`, `:20`, and `:49` define the interaction kinds, stats bucket, and main profiler in one module.
  - `:84-199` exposes many small counter mutators while `:199-320` concentrates the full reporting and reset cycle inside `record_redraw(...)`.
  - `:356-397` duplicates the public profiler API again for the non-`gui-performance` no-op implementation.
  - A targeted repository search for `NativeVelloProfiler`, `record_redraw`, and `add_interaction_latency` only returned production/runtime references, with no focused tests for reporting/reset behavior.
- Recommended change: keep the runtime-facing profiler API stable, but split the feature-gated stats types, reporting/reset logic, and no-op shim into smaller helpers, adding direct tests to the pure reporting/reset pieces where practical.
- Expected impact: profiling changes become safer to review, and drift between the real and no-op implementations becomes easier to detect.
- Risks / tradeoffs: profiling is hot-path-adjacent; the split should not introduce measurable overhead when the feature is enabled.
- Dependencies: none
- Suggested validation:
  - targeted native-Vello runtime/profiling tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. Should `vendor/radiant/src/app/actions/mod.rs` remain one intentionally centralized compatibility surface?

- Question: is the large `UiAction` enum still intentionally centralized, or should future cleanup treat its size as normal file-size debt?
- Evidence:
  - `vendor/radiant/src/app/actions/mod.rs:1-9` explicitly says the module is intentionally broad and should remain the single compatibility surface between runtime and host bridge.
  - The same file is still one of the largest live Rust modules in the tree.
- Why this matters: a future cleanup pass could misread size pressure alone as justification for splitting a compatibility-sensitive contract.
- Affected files/modules: `vendor/radiant/src/app/actions/mod.rs`, `src/app_core/actions`, native-shell action emission, runtime action routing.
- Risk if guessed incorrectly: premature splitting could create unnecessary bridge churn and duplicate contract logic across runtime, host bridge, and automation catalog code.
- Most conservative provisional assumption: keep one top-level `UiAction` surface unless a concrete bridge-contract mismatch appears that cannot be handled with internal helper modules.

### [!] 2. Should `src/selection/range.rs` continue to keep geometry, fades, and gain evaluation together?

- Question: is the dense waveform-selection math still one cohesive domain model, or is there now enough evidence to split fade math away from selection geometry?
- Evidence:
  - `src/selection/range.rs:1-5` explicitly documents that normalized bounds, fade parameters, and fade/gain evaluation rules intentionally live together as one waveform-editing domain model.
  - The file still carries creation, mutation, shift, and fade-evaluation behavior in one module, so size pressure will continue to tempt cleanup passes.
- Why this matters: the repo has an explicit local argument for cohesion here, and size alone is weak evidence for breaking apart a shared domain contract.
- Affected files/modules: `src/selection/range.rs`, waveform editing, selection preview, destructive edit flows, and any controller/runtime code that consumes `SelectionRange`.
- Risk if guessed incorrectly: over-splitting could scatter one stable waveform-editing contract across multiple helpers with little practical safety gain.
- Most conservative provisional assumption: keep `SelectionRange` and its fade/gain math together unless a clearer ownership boundary or recurring testability problem emerges.

## Rejected Ideas

### [-] 1. Promote desktop AIV coverage into normal CI right now

- Why it was considered: the repository has invested heavily in semantic GUI automation and AIV wrappers.
- Why it was rejected: `docs/gui_test_platform.md:189` and `:198` still explicitly say foreground/focus instability keeps desktop AIV local-only and not ready for CI promotion.
- What evidence was missing: repeated local evidence that focus recovery is stable enough to stop treating desktop AIV as a local/manual loop.

### [-] 2. Split `vendor/radiant/src/app/actions/mod.rs` immediately

- Why it was considered: it remains one of the largest live Rust files.
- Why it was rejected: the module itself explicitly documents that the centralized action surface is intentional compatibility structure, not accidental sprawl.
- What evidence was missing: a concrete runtime/host-bridge contract problem that a split would solve better than internal helper organization.

### [-] 3. Split `src/selection/range.rs` immediately

- Why it was considered: the file remains dense and near the repo's preferred size target.
- Why it was rejected: the file-level docs make an explicit cohesion argument, and this audit did not find a stronger ownership or correctness problem than size pressure alone.
- What evidence was missing: a recurring maintenance bug, unclear boundary, or testability problem that clearly justifies separating selection geometry from fade/gain evaluation.
