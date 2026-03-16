# Improvement Audit Plan

- Generated: `2026-03-15`
- Status: `Phase 2 complete`
- Branch: `next`
- Audit baseline commit: `57b06720`
- Canonical Windows validation commands:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Repository Context

- Project purpose: `sempal` is a realtime-oriented sample triage and curation tool centered on exploration, auditioning, and trustworthy library maintenance.
  - Intent label: Explicitly documented
  - Evidence: `README.md`, `docs/design_principles.md`
- Maturity level: a mature Rust workspace with enforced docs/lint/test guardrails, a vendor-owned native GUI framework, and multiple completed execution-plan lanes.
  - Intent label: Strongly implied by code/docs
  - Evidence: `Cargo.toml`, `.github/workflows/ci.yml`, `docs/TEST.md`, `docs/plans/index.md`
- Primary languages / frameworks / tooling: Rust 2024 workspace, `vendor/radiant` native GUI/runtime, Vello/Winit rendering, rusqlite-backed source DBs, nextest, and PowerShell/Bash wrapper scripts.
  - Intent label: Explicitly documented
  - Evidence: `Cargo.toml`, `README.md`, `docs/ARCHITECTURE.md`, `docs/TEST.md`
- Repository shape: domain/controller logic in `src/`, app-core/native projection in `src/app_core`, GUI/runtime framework code in `vendor/radiant`, workspace tools under `tools/` and `apps/`, and planning/docs state under `docs/plans` plus `tmp/`.
  - Intent label: Explicitly documented
  - Evidence: `docs/ARCHITECTURE.md`, `docs/README.md`, `Cargo.toml`
- Architectural boundaries: `src/` owns domain behavior and legacy controller flows, `src/app_core` owns stable action/projection bridging, and `vendor/radiant` owns layout, hit testing, retained shell state, and native runtime orchestration.
  - Intent label: Explicitly documented
  - Evidence: `README.md`, `docs/ARCHITECTURE.md`
- Test strategy: fast compile loops via `devcheck`, pre-push quick validation via `ci_quick`, broader parity via `ci_local`, direct `vendor/radiant` suites, and a GUI contract stack that is broader than default CI but still intentionally layered.
  - Intent label: Explicitly documented
  - Evidence: `docs/TEST.md`, `.github/workflows/ci.yml`, `docs/gui_test_platform.md`
- Documented priorities: responsiveness, non-blocking execution, deterministic interaction, explicit ownership, and gradual burn-down of oversized file/test hubs.
  - Intent label: Explicitly documented
  - Evidence: `docs/design_principles.md`, `docs/QUALITY_SCORE.md`, `docs/file_size_budget_allowlist.txt`
- Explicit non-goals: not a DAW replacement, not a platform/ecosystem, and not a visually ornamental app at the expense of responsiveness.
  - Intent label: Explicitly documented
  - Evidence: `docs/design_principles.md`

## Ordered ROI Backlog

### [x] 1. Refresh the stale cleanup-debt tracking artifacts before relying on them for further prioritization
- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the repo’s debt-tracking docs now materially disagree with the current tree, which makes future prioritization and guardrail reasoning less trustworthy.
- Evidence:
  - `docs/QUALITY_SCORE.md` says the live allowlist matches the current oversized scope.
  - `docs/file_size_budget_allowlist.txt:18`, `:24`, `:52`, and `:54` still list `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs`, `src/app/state/browser.rs`, `vendor/radiant/src/gui_runtime/native_vello/runtime_input.rs`, and `vendor/radiant/src/gui_runtime/native_vello/runtime_startup.rs`.
  - The current tree no longer matches those entries: `browser_search_worker/pipeline/stages.rs` is 15 lines, `src/app/state/browser.rs` is 71 lines, `runtime_input.rs` is 5 lines, and `runtime_startup.rs` is 5 lines.
  - `tmp/cleanup_plan.md:23`, `:31`, and `:79` still point at `src/app/controller/playback/tagging.rs`, `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs`, and `apps/updater-helper/src/ui.rs`, which no longer exist in the current tree.
- Recommended change: refresh `docs/file_size_budget_allowlist.txt`, regenerate the hotspot snapshot, and update the parked cleanup-plan references or label them more explicitly as historical so they are not mistaken for live debt.
- Expected impact: more trustworthy planning inputs, less audit drift, and fewer false positives when choosing the next cleanup lane.
- Risks / tradeoffs: low; the main risk is spending too much time rewriting historical notes instead of updating only the high-signal stale entries.
- Dependencies: none
- Suggested validation:
  - confirm current line counts for the refreshed allowlist entries
  - rerun the file-size/hotspot scan used to regenerate the snapshot
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-16`
- Commit hash: `da07fc16`
- Assumption used: the parked cleanup plan should remain historical, while the live allowlist and hotspot snapshot should match the current tree.
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/prune_file_size_budget_allowlist.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 2. Split `vendor/radiant/src/gui/native_shell/state/frame_build/chrome/sidebar.rs` by sidebar rendering family
- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: sidebar rendering is a user-visible shell path, but one 427-line `render_sidebar` function still owns source header text, add-button chrome, source rows, folder header/recovery badge, folder rows, and footer rendering in one place.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/frame_build/chrome/sidebar.rs` is 427 lines and exposes only `render_sidebar` at line 3.
  - `vendor/radiant/src/gui/native_shell/state/frame_build/chrome/sidebar.rs:18`, `:172`, and `:385` show the same function coordinating `source_add_button_rect`, `compute_sidebar_folder_header_layout`, and `compute_sidebar_footer_text_layout`.
  - `vendor/radiant/src/gui/native_shell/state/tests/sidebar.rs` and `vendor/radiant/src/gui/native_shell/state/tests/browser_toolbar.rs` provide surrounding behavior coverage, but the production ownership surface is still one large render hub with no local tests beside it.
- Recommended change: extract focused source-header/source-row/folder-header/folder-row/footer render helpers or modules, preserve the existing visual contract, and add targeted sidebar render assertions around recovery-badge and row-selection behavior.
- Expected impact: smaller blast radius for sidebar changes and clearer separation between header chrome, row rendering, and footer/status copy.
- Risks / tradeoffs: moderate; row colors, badge placement, and truncation behavior are visually subtle and need characterization coverage.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml sidebar -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_toolbar -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-16`
- Commit hash: `88fb4e3b` (`vendor/radiant`)
- Assumption used: the sidebar visual contract should stay stable while header chrome, source rows, folder rendering, and footer controls move into separate ownership modules.
- Validation outcome:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml sidebar -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_toolbar -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 3. Decompose `vendor/radiant/src/gui_runtime/native_vello/input.rs` into clearer keyboard, shell-hit-test, waveform, and wheel route layers
- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: native input routing is latency-sensitive and central to deterministic interaction, but one 489-line module still mixes keyboard gating, shell-node pointer dispatch, waveform drag-mode glue, geometry helpers, and wheel translation.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/input.rs` is 489 lines.
  - `vendor/radiant/src/gui_runtime/native_vello/input.rs:88`, `:232`, `:320`, `:501`, and `:520` show `action_from_key`, `action_from_pointer_with_motion`, `waveform_action_from_pointer`, `browser_wheel_row_delta`, and `waveform_wheel_zoom_action` still concentrated in one file.
  - The repo already split deeper waveform helpers into `input/waveform_geometry.rs`, `input/waveform_handles.rs`, `input/waveform_routing/`, and `input/wheel.rs`, which strongly implies the remaining top-level dispatch hub is the next ownership seam.
  - Existing coverage is spread across `vendor/radiant/src/gui_runtime/native_vello/tests/key_bindings.rs`, `tests/browser_pointer.rs`, `tests/waveform_pointer.rs`, and `tests/waveform_fades.rs`, not directly around the route-family boundaries in the production module.
- Recommended change: keep the runtime API stable while separating keyboard dispatch, shell-node pointer dispatch, waveform-route glue, and wheel helpers into focused modules with direct route-level tests or tighter test ownership.
- Expected impact: safer edits in the hot input path and clearer responsibility boundaries between shell hit-testing and waveform/browser action translation.
- Risks / tradeoffs: moderate; modifier handling and pointer-route precedence are correctness-sensitive.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests::key_bindings -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests::browser_pointer -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests::waveform_pointer -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-16`
- Commit hash: `0218d0e6` (`vendor/radiant`)
- Assumption used: `input.rs` should remain the stable export surface while key dispatch and pointer route arbitration move into separate ownership modules.
- Validation outcome:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests::key_bindings -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests::browser_pointer -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests::waveform_pointer -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 4. Separate browser-truncation caches, overlay fingerprints, and segmented emit plumbing in `vendor/radiant/src/gui/native_shell/state/cache_types.rs`
- Classification: Architecture improvement
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: retained shell state depends on these cache/fingerprint types, but one file still mixes browser truncation keys, waveform/browser hit-test keys, overlay fingerprints, segmented-frame emit sinks, and LRU-style truncation cache behavior.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/cache_types.rs` is 487 lines.
  - `vendor/radiant/src/gui/native_shell/state/cache_types.rs:40`, `:78`, `:200`, `:337`, `:413`, and `:473` show `BrowserRowTruncationCacheKey`, `WaveformToolbarHitTestCacheKey`, `StateOverlayFingerprint`, `StaticFrameSegments`, `SegmentedStaticEmitContext`, and `impl BrowserRowTruncationCache` in one file.
  - `vendor/radiant/src/gui/native_shell/state.rs` and `model_sync.rs` consume overlay fingerprints, while `browser_rows/truncation.rs` and `hit_testing/waveform.rs` consume cache keys, which points to multiple ownership families sharing one module.
  - The file has a `#[cfg(test)]`-gated struct field surface, but no local `mod tests` block covering the cache or segmented-emitter behavior directly.
- Recommended change: split the file into focused modules for browser truncation caches, overlay fingerprints, and segmented frame-emitter plumbing, then add direct tests for truncation-cache eviction/touch and segmented sink routing where behavior is non-obvious.
- Expected impact: clearer cache ownership, smaller review surfaces, and easier targeted testing around retained-shell invalidation behavior.
- Risks / tradeoffs: moderate; cache-key drift or segmented-emitter mistakes could cause stale or missing shell segments.
- Dependencies: none
- Suggested validation:
  - targeted `vendor/radiant` tests for browser truncation and shell frame segmentation
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui::native_shell -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-16`
- Commit hash: `360d3ba6` (`vendor/radiant`)
- Assumption used: cache-type families should split along existing consumers without changing the small subset of shell/runtime fingerprints that intentionally stay crate-visible.
- Validation outcome:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui::native_shell -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 5. Split `src/app/controller/playback/loop_crossfade.rs` into controller orchestration, file-output helpers, and undo registration
- Classification: Architecture improvement
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: loop-crossfade is a file-writing user flow, but one 470-line module still combines prompt setup, sample decoding, cut-frame selection, output naming, WAV writing, DB/browser registration, and deferred undo job assembly.
- Evidence:
  - `src/app/controller/playback/loop_crossfade.rs` is 470 lines.
  - `src/app/controller/playback/loop_crossfade.rs:11` and `:25` expose `request_loop_crossfade_prompt_for_browser_row` and `apply_loop_crossfade_prompt`.
  - The same file also owns `write_loop_crossfade_wav` at `:269` and `maybe_capture_loop_crossfade_undo` at `:319`.
  - Local tests exist at `:384`, but they currently cover the DSP helper path plus the happy-path prompt/apply flow; the ownership boundary between file output, registration, and undo remains broad.
- Recommended change: keep the user-facing behavior stable while separating prompt/apply orchestration from pure crossfade math, file-output naming/writing, and undo payload assembly; add a few focused controller-flow edge tests where the split exposes them naturally.
- Expected impact: safer future edits to a destructive-adjacent flow and clearer reasoning about which layer owns audio rewriting versus undo registration.
- Risks / tradeoffs: moderate; filename collision, tag preservation, and deferred undo semantics must stay exact.
- Dependencies: none
- Suggested validation:
  - `cargo test loop_crossfade -- --test-threads=1`
  - `cargo test app::controller::playback::tests -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-16`
- Commit hash: `668b9c04`
- Assumption used: the root playback module should keep prompt/apply orchestration while pure audio prep, file registration, and deferred undo capture move behind it.
- Validation outcome:
  - `cargo test loop_crossfade -- --test-threads=1`
  - `cargo test app::controller::playback::tests -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 6. Split `vendor/radiant/src/gui/native_shell/style/sizing.rs` into base tokens, tier deltas, and sizing invariants
- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters: layout sizing is a compatibility surface used across most native-shell layout/state helpers, but one 580-line file still combines the huge `SizingTokens` schema, base values, compact/wide mutations, and helper methods.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/style/sizing.rs` is 580 lines.
  - `vendor/radiant/src/gui/native_shell/style/sizing.rs:7` defines the large `SizingTokens` contract, while `:207`, `:217`, `:320`, `:403`, and `:486` hold `sizing_for_tier`, `base_sizing`, `apply_compact_sizing`, `apply_wide_sizing`, and `impl SizingTokens`.
  - The token pack is consumed broadly across `layout_adapter/`, `state/`, and `layout_runtime.rs`, which means small edits have a wide blast radius.
  - Current tests live in `vendor/radiant/src/gui/native_shell/style/mod.rs`, not next to the token pack or its tier-delta logic.
- Recommended change: separate the base token schema from compact/wide overrides and add a small local invariant suite for non-negative sizes, tier monotonicity where intended, and minimum-width/height assumptions that layout code relies on.
- Expected impact: easier review of layout-token changes and less accidental churn in the central shell sizing contract.
- Risks / tradeoffs: medium; some token relationships may be intentionally asymmetric across tiers, so invariants should stay minimal and evidence-based.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui::native_shell::style -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui::native_shell::tests::chrome_layout -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-16`
- Commit hash: `964086a6` (`vendor/radiant`)
- Assumption used: the giant sizing file should keep the stable `SizingTokens` schema while base values, tier deltas, and local invariants move into separate ownership files.
- Validation outcome:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui::native_shell::style -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui::native_shell::state::tests::chrome_layout -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 7. Break the remaining oversized native-shell test hubs into ownership-aligned modules
- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: several of the largest remaining files are regression hubs rather than production code, and they currently mix unrelated layout, toolbar, prompt, sidebar, and canonical-frame assertions in ways that make failures harder to localize.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/mod.rs` is 900 lines and mixes `canonical_shell_model` at `:35` with toolbar hit tests at `:379`, prompt hit tests at `:541`, source-action tests at `:694`, and canonical-frame assertions at `:913`.
  - `vendor/radiant/src/gui/native_shell/state/tests/browser_toolbar.rs` is 528 lines and mixes source-header, browser search, rating-filter, action-strip, and top-bar coverage.
  - `vendor/radiant/src/gui/native_shell/state/tests/overlays.rs` is 468 lines and mixes waveform hover, toolbar overlays, browser-row stale-state checks, and generic hover cleanup.
- Recommended change: split these hubs by behavior family, keep a shared canonical-shell fixture helper where necessary, and align each test module with the production ownership boundary it protects.
- Expected impact: more discoverable failures, smaller diffs for regression additions, and less accumulation pressure in omnibus test files.
- Risks / tradeoffs: low; mostly structural, but shared fixtures should stay simple so targeted test commands remain obvious.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui::native_shell -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-16`
- Commit hash: `b425a971` (`vendor/radiant`)
- Assumption used: the native-shell regression coverage should keep one shared canonical fixture helper while each behavior family moves into its own local test module.
- Validation outcome:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui::native_shell -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 8. Split `vendor/radiant/src/gui/native_shell/shots.rs` into fixture I/O, snapshot canonicalization, rasterization, and model builders
- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Low-Medium
- Effort: M
- Why it matters: visual-regression maintenance still flows through one 642-line file that mixes snapshot serialization, JSON canonicalization, software rasterization, fixture compare/write logic, and scenario model builders.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/shots.rs` is 642 lines.
  - `vendor/radiant/src/gui/native_shell/shots.rs:135`, `:193`, `:237`, `:349`, `:431`, `:489`, and `:579` show `build_snapshot`, `canonicalize_json`, `rasterize_shot`, `write_or_compare_shot`, and the startup/browser/waveform fixture builders in one module.
  - This file sits on the critical path for trusted shot-fixture updates, but the ownership boundary between fixture plumbing and model construction is still broad.
- Recommended change: separate snapshot/fixture mechanics from scenario model builders, and keep the canonicalization/rasterization helpers reusable without pulling all model fixtures into the same file.
- Expected impact: easier maintenance of visual-regression fixtures and smaller diffs when only one fixture family changes.
- Risks / tradeoffs: medium; fixture bytes and update semantics must stay stable so reviewers can trust shot diffs.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml startup_shot_matches_fixture -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_dense_shot_matches_fixture -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml waveform_selection_shot_matches_fixture -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-16`
- Commit hash: `c4800627` (`vendor/radiant`)
- Assumption used: visual-regression fixture bytes are part of the trusted contract, so the split must preserve exact snapshot and raster output while moving fixture I/O, canonicalization, rasterization, and model builders into separate helpers.
- Validation outcome:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml startup_shot_matches_fixture -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_dense_shot_matches_fixture -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml waveform_selection_shot_matches_fixture -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

## Open Questions / Missing Definitions

### [!] 1. Should the parked cleanup backlog be kept continuously current, or intentionally remain a historical snapshot?
- Question: should `tmp/cleanup_plan.md` be refreshed whenever major cleanup items land, or is it intentionally a dated archive that should not be reused directly?
- Evidence:
  - `tmp/cleanup_plan.md` explicitly says it is parked, but it still contains active-looking `[ ]` items with paths that no longer exist, including `src/app/controller/playback/tagging.rs`, `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs`, and `apps/updater-helper/src/ui.rs`.
  - `AGENTS.md`, `MEMORY.md`, and `docs/plans/index.md` currently point readers at `tmp/improvement_audit_plan.md` as the live source of truth, not `tmp/cleanup_plan.md`.
- Why this matters: the safest remediation differs depending on whether the parked cleanup plan should be corrected or simply relabeled as historical-only.
- Affected files/modules:
  - `tmp/cleanup_plan.md`
  - `docs/QUALITY_SCORE.md`
  - `docs/file_size_budget_allowlist.txt`
- Risk if guessed incorrectly: either wasted cleanup of a historical archive or continued drift in a document that maintainers may still treat as live debt.
- Most conservative provisional assumption: keep the parked plan historical, but refresh the live allowlist/hotspot artifacts and make the historical status unmistakable.

### [!] 2. Is `vendor/radiant/src/app/actions.rs` intentionally meant to stay centralized as one schema file?
- Question: should the native `UiAction` catalog remain a single compatibility surface, or is there an intended direction toward family-specific action declarations plus a shared export layer?
- Evidence:
  - `vendor/radiant/src/app/actions.rs` is still 450 lines and contains the whole serialized `UiAction` surface.
  - Earlier audit work explicitly rejected splitting it because the repo evidence favored centralization.
- Why this matters: action-schema centralization affects whether later runtime/native-shell cleanup should keep routing logic consolidated around one giant action enum or prepare for family-based modules.
- Affected files/modules:
  - `vendor/radiant/src/app/actions.rs`
  - `src/app_core/actions/catalog/`
  - `docs/gui_test_platform.md`
- Risk if guessed incorrectly: unnecessary churn in a compatibility surface used by runtime code, GUI tooling, and action catalogs.
- Most conservative provisional assumption: treat `UiAction` as intentionally centralized unless a narrower evidence trail emerges.

### [!] 3. Should native-shell regression tests stay organized around one canonical-shell fixture, or follow production ownership boundaries more strictly?
- Question: is the current large `vendor/radiant/src/gui/native_shell/mod.rs` test hub deliberate because the shell contract is inherently cross-cutting, or is it just accumulated debt?
- Evidence:
  - `vendor/radiant/src/gui/native_shell/mod.rs` mixes layout, toolbar, prompt, source action, and canonical-frame assertions around one `canonical_shell_model` helper.
  - Separate focused suites already exist under `state/tests/`, which suggests at least partial appetite for more ownership-aligned test modules.
- Why this matters: the safest cleanup is different if the big module is intended to preserve one top-level shell contract view.
- Affected files/modules:
  - `vendor/radiant/src/gui/native_shell/mod.rs`
  - `vendor/radiant/src/gui/native_shell/state/tests/*`
- Risk if guessed incorrectly: over-splitting tests could make cross-shell regressions harder to detect or duplicate fixture setup unnecessarily.
- Most conservative provisional assumption: keep one shared canonical-shell fixture helper, but move behavior families into smaller modules around it.

## Rejected Ideas

### [-] 1. Reopen the previously completed browser-search/browser-state/runtime-startup audit items
- Why it was considered: older audit artifacts and the stale allowlist still mention those files prominently.
- Why it was rejected: the current tree shows those paths have already been split down substantially (`browser_search_worker/pipeline/stages.rs` is 15 lines, `src/app/state/browser.rs` is 71 lines, `runtime_input.rs` is 5 lines, `runtime_startup.rs` is 5 lines).
- What evidence was missing: no current oversized ownership surface or fresh contradictory behavior evidence remained in those specific wrapper files.

### [-] 2. Prioritize `src/analysis/frequency_domain/stft.rs` immediately
- Why it was considered: it is still 431 lines and over the nominal 400-line budget.
- Why it was rejected: the file has a clear single responsibility, local tests in the file itself, and no adjacent docs/tests indicating the same level of ownership ambiguity as the GUI/runtime/sidebar candidates above.
- What evidence was missing: no repo note, stale contract, or nearby mixed-responsibility symptoms suggested it is a top-leverage split right now.

### [-] 3. Promote desktop AIV automation into default CI
- Why it was considered: GUI automation is an active, documented repository direction.
- Why it was rejected: `docs/gui_test_platform.md` still calls out Windows foreground/focus instability and explicitly keeps the desktop-AIV loop local-only.
- What evidence was missing: no current repository evidence that the desktop AIV layer is stable enough for default CI promotion.
