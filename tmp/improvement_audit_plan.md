# Evidence-Driven Improvement Audit Plan

- Generated (UTC): `2026-03-15T09:37:47Z`
- Repository: `C:\dev\sempal`
- Branch / head: `next` / `6c9dc2d8`
- Phase: `Phase 2 in progress`
- Status: `Implementing the ROI-ranked backlog in plan order; items 1-2 are complete.`
- Validation baseline:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` passed on this branch.
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` regenerated `tmp/cleanup_audit_hotspots.md`.

## Repository Context

- Project purpose: Rust desktop sample-triage and sample-editing tool focused on responsive audio auditioning, curation, and library cleanup.
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `docs/design_principles.md`
- Maturity level: active, guardrail-heavy application with live companion apps/tools, strong local validation scripts, and ongoing GUI/runtime migration and automation work.
  - Confidence: Strongly implied by code/docs
  - Evidence: `Cargo.toml`, `.github/workflows/ci.yml`, `docs/gui_test_platform.md`, `docs/QUALITY_SCORE.md`
- Primary languages/frameworks/tooling: Rust 2024 workspace, Cargo, PowerShell-first Windows workflow wrappers, and the local `vendor/radiant` GUI/runtime layer.
  - Confidence: Explicitly documented
  - Evidence: `Cargo.toml`, `README.md`, `AGENTS.md`
- Repository shape: root app in `src/`, companion apps in `apps/`, support tools in `tools/`, developer docs in `docs/`, live plans in `tmp/`, and GUI framework code in `vendor/radiant/`.
  - Confidence: Explicitly documented
  - Evidence: `Cargo.toml`, `docs/ARCHITECTURE.md`
- Architectural boundaries: domain/application logic belongs in `src/**`, backend-neutral projection/action contracts in `src/app_core/**`, and GUI behavior/layout/input/runtime internals in `vendor/radiant/**`.
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `docs/ARCHITECTURE.md`
- Test strategy: narrow PowerShell wrappers for the fast loop, broader local parity via `ci_local`, and explicit semantic GUI contract/AIV loops beside normal Rust unit and integration tests.
  - Confidence: Explicitly documented
  - Evidence: `docs/TEST.md`, `docs/gui_test_platform.md`, `scripts/ci_quick.ps1`, `scripts/run_gui_contract.ps1`
- Canonical local validation on Windows:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `AGENTS.md`, `docs/README.md`, `docs/TEST.md`
- Documented priorities: responsiveness, non-blocking work, deterministic interaction, explicit ownership boundaries, and strong documentation/guardrails.
  - Confidence: Explicitly documented
  - Evidence: `docs/design_principles.md`, `AGENTS.md`
- Explicit non-goals: DAW replacement, attention-capture platform behavior, and speculative large-scope redesigns without evidence.
  - Confidence: Explicitly documented
  - Evidence: `docs/design_principles.md`

## ROI-Ranked Backlog

### 1. [x] Promote the semantic GUI contract loop into the normal quick validation path

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters:
  - The repository now treats GUI actions, semantic automation nodes, and GUI artifacts as contract surfaces, but the default quick gate still skips the dedicated GUI contract loop. That leaves the newest migration surface under-validated during normal iteration.
- Evidence:
  - `docs/gui_test_platform.md` says the GUI test platform exists to make native GUI actions cataloged, semantically testable, and consumable by automation.
  - `docs/gui_test_platform.md` lists “Promote the GUI contract loop into `ci_quick` once it is stable enough to be mandatory” as an immediate next step.
  - `scripts/run_gui_contract.ps1` already exists and runs catalog tests, `gui_test` tests, and a focused `vendor/radiant` hit-test smoke.
  - `scripts/ci_quick.ps1` currently only runs `cargo nextest run -p sempal --profile quick --lib --tests`.
- Recommended change:
  - Add `scripts/run_gui_contract.ps1` to `scripts/ci_quick.ps1` after the existing branch-policy check.
  - Keep desktop AIV loops out of `ci_quick`; only the semantic/runtime contract slice should become mandatory here.
- Expected impact:
  - Catches action-catalog, semantic-node, and runtime-hit-test regressions before full CI.
  - Aligns the quick loop with the repo’s documented GUI testing direction.
- Risks / tradeoffs:
  - Could lengthen the quick loop slightly.
  - If the contract lane still has intermittent failures, it will raise local friction until stabilized.
- Dependencies:
  - None.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-15`
- Implementation commit: `2fddca31`
- Assumptions used:
  - `ci_quick` should adopt only the semantic/runtime GUI contract lane; the broader desktop AIV automation loop remains outside the default quick gate.
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed on `2026-03-15`, including `scripts/run_gui_contract.ps1`.
- Plan-order deviation: None

### 2. [x] Make browser multi-selection state single-source-of-truth and derive the compatibility view lazily

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - Browser selection is now tracked in both absolute-index and relative-path form. The controller repeatedly resynchronizes both views, bumps revisions off that sync work, and re-derives markers and projection lookups from the duplicated state. That adds maintenance risk to one of the repo’s busiest paths.
- Evidence:
  - `src/app/state/browser.rs` stores both `selected_indices` and `selected_paths`, plus a shared `selected_paths_revision`.
  - `src/app/controller/library/wavs/browser_actions/selection.rs` has `sync_browser_selected_indices_from_paths`, `sync_browser_selected_paths_from_indices`, `mark_browser_selected_paths_changed`, and `set_browser_selected_indices`.
  - `src/app/controller/library/wavs/browser_lists.rs` clones and resynchronizes both representations inside `prune_browser_selection()` before every rebuild/marker refresh.
  - `src/app_core/native_shell/browser_projection/cache.rs` and `src/app_core/native_bridge/projection_cache/projection_key/browser.rs` consume revisions/lengths from this mixed state.
- Recommended change:
  - Keep one canonical browser multi-selection representation, preferably absolute indices because the current code already treats them as primary in several comments and projection paths.
  - Derive relative paths only at explicit I/O boundaries such as clipboard/export/file-op requests, or behind one cached adapter.
  - Keep a single revision source tied to the canonical selection state.
- Expected impact:
  - Reduces selection drift risk and controller churn in browser rebuilds.
  - Makes later browser projection/cache work easier to reason about.
- Risks / tradeoffs:
  - Selection, clipboard, drag/drop, and browser projection all touch this state.
  - Any hidden consumer that assumes eager `selected_paths` updates will need targeted coverage.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted browser selection tests
  - Clipboard selection tests
  - Browser projection cache tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-15`
- Implementation commit: `7338908a`
- Assumptions used:
  - Relative paths are the safest canonical browser-selection identity because they survive rename/move remapping and projection reorder better than absolute entry indices.
- Validation outcome:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed on `2026-03-15`.
  - `cargo test browser_selection -- --test-threads=1` passed on `2026-03-15`.
  - `cargo test browser_cache -- --test-threads=1` passed on `2026-03-15`.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed on `2026-03-15`.
- Plan-order deviation: None

### 3. [ ] Decompose `browser_search.rs` into scoring/cache mechanics, async policy, and UI-trigger handlers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - Browser search remains one of the most active and performance-sensitive controller paths. One file still owns cache state, synchronous scoring, label cache fill, async job dispatch, search/filter/sort mutations, runtime environment toggles, and test-only pipeline overrides.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/library/wavs/browser_search.rs` at 474 lines.
  - `src/app/controller/library/wavs/browser_search.rs` defines `BrowserSearchCache`, scoring helpers, `dispatch_search_job`, filter/rating/sort/search handlers, environment-based async/offload policy, and a test-only thread-local override in one module.
  - The same file is responsible for both runtime behavior and test-only pipeline forcing (`with_browser_async_pipeline_enabled_for_tests`).
- Recommended change:
  - Split the file into focused modules such as `cache`, `dispatch_policy`, and `mutations`.
  - Keep the external controller façade stable while reducing the number of unrelated reasons to edit the same file.
  - Preserve the current async-authoritative runtime policy unless a later clarification explicitly changes it.
- Expected impact:
  - Lowers regression risk in a hot controller surface.
  - Makes search behavior changes more reviewable and easier to test in isolation.
- Risks / tradeoffs:
  - Search behavior is user-visible and performance-sensitive.
  - Careless decomposition could hide the current explicit async-vs-sync behavior.
- Dependencies:
  - None.
- Suggested validation:
  - Browser async tests in `src/app/controller/tests/browser_async.rs`
  - Browser filter/search tests in `src/app/controller/tests/browser_core/filters.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 4. [ ] Split browser projection refresh and lookup-map maintenance out of `browser_lists.rs`

- Classification: Refactor / cleanup
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - `browser_lists.rs` is now the place where selection pruning, visible-row projection application, focus/loaded marker refresh, viewport sync, and lazy reverse-lookup rebuilding all meet. That centrality makes even behavior-preserving changes risky.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/library/wavs/browser_lists.rs` at 461 lines.
  - `src/app/controller/library/wavs/browser_lists.rs` owns `rebuild_browser_lists`, `apply_browser_projection`, `prune_browser_selection`, `refresh_browser_selection_markers`, and both visible/triage lookup rebuild paths.
  - The file duplicates highlight/focus/loaded-row logic across full rebuild and marker-only refresh.
- Recommended change:
  - Separate projection application, selection-prune normalization, and lookup-map rebuild logic into focused helpers/modules.
  - Leave `rebuild_browser_lists()` and marker refresh as thin orchestration entrypoints.
- Expected impact:
  - Reduces the blast radius for future browser-view maintenance.
  - Makes marker refresh and reverse lookup invariants easier to characterize directly.
- Risks / tradeoffs:
  - This file sits between controller state and native/browser projection behavior.
  - Refactors must not perturb visible-row revision semantics or viewport stability.
- Dependencies:
  - Item 2 should inform the final ownership split if both are approved.
- Suggested validation:
  - Existing lookup tests in `src/app/controller/library/wavs/browser_lists.rs`
  - Browser selection/focus tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 5. [ ] Separate folder-browser scan/cache orchestration from tree projection and fuzzy filtering

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The folder browser is a mixed controller/UI cache with hotkeys, manual-folder tracking, disk-scan freshness, row-tree building, and fuzzy search ranking in one place. That makes the folder feature harder to extend safely than the existing tests suggest.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/library/source_folders/tree.rs` at 442 lines.
  - `src/app/controller/library/source_folders/tree.rs` defines `FolderBrowserModel`, disk-refresh application, timed refresh orchestration (`AUTO_SYNC_INTERVAL`), row-tree flattening, fuzzy row filtering, and recursive disk scanning.
  - Other folder features (`actions/hotkeys.rs`, `selection/navigation.rs`, `selection/search.rs`) already depend on the same `FolderBrowserModel`, which increases coupling pressure.
- Recommended change:
  - Move disk-scan freshness/cache behavior into a focused model module.
  - Keep tree building/flattening and fuzzy filtering in separate projection helpers.
  - Preserve the current `refresh_folder_browser_for_tests()` seam so existing tests stay deterministic.
- Expected impact:
  - Makes folder-browser behavior easier to reason about without changing user-visible semantics.
  - Reduces the chance that scan-policy changes accidentally disturb row projection behavior.
- Risks / tradeoffs:
  - Folder selection and hotkey flows already depend on this state model.
  - Splitting without clear ownership boundaries could just move complexity around.
- Dependencies:
  - None.
- Suggested validation:
  - Folder tests under `src/app/controller/tests/folders_core/`
  - `src/app/controller/tests/folders_search.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 6. [ ] Add direct hotkey-registry invariants and reduce reliance on ad hoc registration tests

- Classification: Test gap
- Confidence: High
- ROI: Medium
- Effort: S
- Why it matters:
  - Hotkeys are a core interaction surface, but the codebase currently proves them mostly through piecemeal action-specific tests instead of a direct registry invariant suite. That leaves duplicate ids, duplicate gestures within a scope, and accidental scope collisions harder to catch than they should be.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/ui/hotkeys/actions.rs` at 455 lines.
  - `src/app/controller/ui/hotkeys/mod.rs` simply filters `HOTKEY_ACTIONS`, so most safety depends on the table staying internally consistent.
  - Existing tests in files like `src/app/controller/tests/focus_random/browser_focus.rs`, `history.rs`, `mutation_hotkeys.rs`, and waveform hotkey tests validate selected actions, not the registry as a whole.
- Recommended change:
  - Add direct table-driven tests for duplicate action ids, duplicate gestures per scope, and representative global/focus lookup behavior.
  - If useful, split the table by scope family to make review smaller, but keep one explicit exported registry.
- Expected impact:
  - Improves confidence in a user-facing control surface with minimal code churn.
  - Reduces future hotkey regressions caused by adding one more entry to a large table.
- Risks / tradeoffs:
  - If the repo intentionally allows some cross-scope gesture reuse, the tests must encode that rule explicitly rather than over-constraining the table.
- Dependencies:
  - None.
- Suggested validation:
  - New hotkey registry invariant tests
  - Existing waveform/browser hotkey controller tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 7. [ ] Clarify and harden the `wav_sanitize` public `Read + Seek` contract

- Classification: Documentation gap
- Confidence: High
- ROI: Medium
- Effort: S-M
- Why it matters:
  - `wav_sanitize.rs` exposes public helpers used by live audio-loading and waveform paths, but its documentation and inline comments no longer describe the contract cleanly. The most complex part of the type is `Seek`, yet the current tests only validate read-through and in-memory sanitization behavior.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/wav_sanitize.rs` at 467 lines.
  - `src/wav_sanitize.rs` implements non-trivial `SeekFrom::Start`, `SeekFrom::Current`, and `SeekFrom::End` logic for `SanitizedWavReader`.
  - The file still contains stale conversational comments such as “Let’s implement Current via Start for simplicity” and “I’ll add a wrapper,” which are not stable contract documentation.
  - Runtime code calls these helpers from `src/app/controller/library/wav_io.rs`, `src/app/controller/playback/audio_loader/stages.rs`, and `src/app/controller/library/wavs/waveform_rendering.rs`.
- Recommended change:
  - Replace conversational/stale comments with stable doc comments that define what gets repaired, what does not, and how `Seek` behaves across sanitized headers.
  - Add direct tests for `SeekFrom::Start`, `SeekFrom::Current`, and `SeekFrom::End` on both pass-through and sanitized-header paths.
  - If the file is still hard to reason about after that, split streaming-reader behavior from header-repair helpers.
- Expected impact:
  - Makes a public audio-loading surface safer to change.
  - Reduces the chance of silent seek-position bugs in downstream decoders.
- Risks / tradeoffs:
  - WAV header repair is format-sensitive; avoid broadening repair scope without stronger corpus evidence.
- Dependencies:
  - None.
- Suggested validation:
  - `wav_sanitize` unit tests
  - Targeted audio/waveform loading tests that exercise sanitized inputs
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 8. [ ] Add targeted coverage for deferred undo/redo file flows and separate generic undo primitives from controller/file glue

- Classification: Test gap
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters:
  - Undo/redo is explicitly documented as a first-class interaction primitive, but the most complex part of the implementation is the deferred file-operation path, not the generic in-memory stack. That path currently spans multiple files with only light indirect coverage.
- Evidence:
  - `docs/design_principles.md` defines universal undo/redo as a first-class interaction primitive.
  - `src/app/controller/undo.rs` mixes generic undo stack logic, controller-facing `undo()` / `redo()` entrypoints, selection undo helpers, and `OverwriteBackup`.
  - `src/app/controller/undo_jobs.rs` runs deferred filesystem undo jobs for overwrite/remove/restore cases.
  - `src/app/controller/ui/file_ops.rs` applies deferred undo results and restores/pushes entries depending on success or cancellation.
  - `src/app/controller/undo.rs` local tests cover stack limit and redo clearing, but do not directly characterize deferred file-job lifecycle behavior.
- Recommended change:
  - Add targeted tests for deferred undo success, cancellation, and failure restoration behavior.
  - Split generic undo structures from controller/file-operation glue if that makes the deferred flow easier to test directly.
- Expected impact:
  - Increases confidence in one of the repo’s explicitly documented trust/safety features.
  - Makes future file-editing features safer to add.
- Risks / tradeoffs:
  - File-backed undo tests can become slow or fragile if they are not tightly scoped.
  - Refactors must preserve current user-facing undo labels and stack behavior.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted undo/deferred file-op tests
  - Existing selection/tagging undo coverage
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 9. [ ] Expand the semantic automation tree to cover the remaining browser action-strip buttons and other micro-controls

- Classification: Feature opportunity
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters:
  - The repo already invested in a semantic GUI automation surface for the CLI runner and AIV desktop loop, but the docs still describe that automation coverage as broad rather than exhaustive. The remaining uncovered controls reduce the value of the existing platform.
- Evidence:
  - `docs/gui_test_platform.md` says automation coverage is “broad but not yet exhaustive for every micro-control.”
  - The same doc lists “Expand the automation tree to cover the remaining browser action-strip buttons and other micro-controls” as the first immediate next step.
  - The platform already has CLI, scenario, runtime-artifact, and AIV wrappers that benefit from stable semantic ids.
- Recommended change:
  - Extend the semantic snapshot only where the current automation/test platform already expects stable node ids.
  - Prioritize browser action-strip controls first, because the docs name them explicitly.
  - Keep scope limited to semantic-node coverage; do not couple this item to desktop AIV CI promotion.
- Expected impact:
  - Increases the utility of the existing GUI automation platform without inventing a new product lane.
  - Makes later GUI regression work less dependent on screenshot matching.
- Risks / tradeoffs:
  - Semantic ids are compatibility surfaces once external tooling depends on them.
  - Expanding coverage without characterization tests could create brittle automation churn.
- Dependencies:
  - Item 1 becomes more valuable once more of the automation tree is contract-tested.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_suite.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] What exactly should become mandatory in `ci_quick`: the semantic GUI contract loop only, or a broader GUI lane?

- Evidence:
  - `docs/gui_test_platform.md` says the semantic contract loop should be promoted into `ci_quick`.
  - The same doc says desktop AIV stability still depends on Windows foreground/focus behavior and that no CI gate yet enforces desktop AIV smoke stability.
- Why this matters:
  - Without a precise gate definition, “promote GUI testing into quick validation” can accidentally pull unstable desktop automation into the default loop.
- Affected files/modules:
  - `scripts/ci_quick.ps1`
  - `scripts/run_gui_contract.ps1`
  - `scripts/run_gui_aiv_*.ps1`
  - `docs/gui_test_platform.md`
- Risk if guessed incorrectly:
  - Either the quick loop stays weaker than intended, or it becomes noisy/slow due to the unstable desktop-AIV slice.
- Most conservative provisional assumption:
  - Promote only `scripts/run_gui_contract.ps1` into `ci_quick`; keep AIV wrappers opt-in.

### [!] Is `selected_paths` still intended to be durable browser state, or only a compatibility view over canonical index-based selection?

- Evidence:
  - `src/app/controller/library/wavs/browser_actions/selection.rs` calls the path list a “compatibility path list” while also keeping `selected_paths_revision`.
  - `src/app/state/browser.rs` still stores both paths and indices.
- Why this matters:
  - The safe refactor boundary depends on whether callers are allowed to assume eager path-state availability.
- Affected files/modules:
  - `src/app/state/browser.rs`
  - `src/app/controller/library/wavs/browser_actions/selection.rs`
  - `src/app/controller/library/wavs/browser_lists.rs`
  - `src/app_core/native_shell/browser_projection/cache.rs`
- Risk if guessed incorrectly:
  - Selection, clipboard, projection caching, or drag/drop behavior could drift subtly.
- Most conservative provisional assumption:
  - Treat absolute indices as authoritative and keep path materialization behind a compatibility adapter until all consumers are explicit.

### [!] What malformed WAV-header variants are intentionally in scope for automatic sanitization?

- Evidence:
  - `src/wav_sanitize.rs` currently repairs padded PCM/IEEE-float `fmt ` chunks, but the public helper names and docs are broad.
- Why this matters:
  - Without a clear scope, future changes can accidentally expand repair heuristics in a risky media-decoding path.
- Affected files/modules:
  - `src/wav_sanitize.rs`
  - `src/app/controller/library/wav_io.rs`
  - `src/app/controller/playback/audio_loader/stages.rs`
  - `src/app/controller/library/wavs/waveform_rendering.rs`
- Risk if guessed incorrectly:
  - The app could silently mutate unsupported files incorrectly or reject cases maintainers assumed were handled.
- Most conservative provisional assumption:
  - Keep the repair scope narrow and document only the currently implemented padded-`fmt ` fixes until broader corpus evidence exists.

### [!] What user-visible freshness contract should the folder browser provide for disk-only folders?

- Evidence:
  - `src/app/controller/library/source_folders/tree.rs` refreshes disk folders on a 10-second interval and reuses cached folder state when entries are empty or a load is pending.
- Why this matters:
  - Scan cadence and cache reuse affect user-visible folder availability, but the behavior is implicit in code rather than documented.
- Affected files/modules:
  - `src/app/controller/library/source_folders/tree.rs`
  - `src/app/controller/tests/folders_*`
- Risk if guessed incorrectly:
  - Cleanup refactors can unintentionally make folder rows feel stale or overly eager to rescan.
- Most conservative provisional assumption:
  - Preserve the current async refresh cadence and cache reuse rules unless a user-visible contract is documented.

## Rejected Ideas

### [-] Remove the synchronous browser rebuild path entirely

- Why it was considered:
  - The runtime path now prefers the async worker pipeline.
- Why it was rejected:
  - The repository still uses the synchronous path for deterministic tests and fallback behavior.
- What evidence was missing:
  - No doc or code comment says the sync path should be deleted outright.

### [-] Start a new crate-split lane immediately for build speed

- Why it was considered:
  - `docs/build_speed.md` lists future split candidates.
- Why it was rejected:
  - The same doc explicitly says to re-measure first and only continue if timing data still justifies more structural splits.
- What evidence was missing:
  - No fresh timing artifact in the current repo snapshot shows the app package still needs another crate split.

### [-] Add user-configurable hotkeys

- Why it was considered:
  - `HOTKEY_ACTIONS` is large and high-visibility.
- Why it was rejected:
  - A large registry is not by itself evidence that customizable keymaps are an intended product direction.
- What evidence was missing:
  - No roadmap note, doc section, or test surface suggests customizable hotkeys are planned.
