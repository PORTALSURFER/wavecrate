# Cleanup Audit Backlog

- Refreshed (UTC): `2026-03-12T12:44:59.1475131Z`
- Sempal branch/head: `next` / `688f0a68`
- Radiant branch/head: `next` / `2cc5c0f4`
- Phase: `Phase 2 in progress`
- Status: `Items 1-3 are complete; continuing strict sequential implementation at item 4`
- Canonical quick gate (Windows): `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Canonical full local CI (Windows): `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Ordered ROI backlog

### 1. [x] Decouple `radiant` crate branding and defaults from Sempal host assumptions
- ROI / Effort: High / S
- Why it matters: `radiant` is intended to be its own reusable GUI crate, but several defaults and fixture labels still hardcode the Sempal host identity, which blurs ownership and makes reuse harder.
- Evidence: `vendor/radiant/src/lib.rs:1` documents `radiant` as being for Sempal; `vendor/radiant/src/gui_runtime/mod.rs:45-56` defaults the native window title to `Sempal`; `vendor/radiant/src/app/shell.rs:194`, `vendor/radiant/src/gui_runtime/native_vello/runtime_startup.rs:388`, and `vendor/radiant/src/gui/native_shell/state/automation.rs:29` also hardcode `Sempal`; test fixtures in `vendor/radiant/src/gui/native_shell/mod.rs:37` and `vendor/radiant/src/gui/native_shell/shots.rs:433,491,579` embed Sempal-specific titles.
- Recommended change: Make crate docs, defaults, semantic labels, and fixture names host-neutral; require the host to supply product branding explicitly where needed instead of baking it into `radiant`.
- Risk / tradeoffs: Low; expect snapshot/test churn and small host call-site adjustments where the old implicit title was relied upon.
- Suggested validation: `cargo test -p radiant`, targeted startup/snapshot tests under `vendor/radiant/src/gui_runtime/native_vello/tests`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Completed: `2026-03-12` - `vendor/radiant` commit `f8063a2b` replaced Sempal-specific default titles, startup placeholder branding, automation root labeling, and radiant-only fixture names/docs with host-neutral `Radiant` defaults while leaving host-supplied branding explicit.

### 2. [x] Refresh stale cleanup metadata, transitional comments, and file-size debt tracking
- ROI / Effort: High / S
- Why it matters: Current cleanup comments and debt trackers still describe already-finished work as pending, which misleads future sessions and weakens the value of the file-size budget.
- Evidence: `src/lib.rs:3-5` still says the map-view split is pending in old cleanup item 19; `vendor/radiant/src/lib.rs:12-16` still references old backlog items 20-23/26/30 as pending; `docs/file_size_budget_allowlist.txt` still lists paths such as `src/app_core/native_bridge.rs`, `src/audio/output.rs`, `src/waveform/render.rs`, `src/waveform/mod.rs`, and `vendor/radiant/src/app/mod.rs` even though those surfaces have already been split or moved.
- Recommended change: Replace stale cleanup references with current ownership notes, prune obsolete allowlist entries, and keep only real remaining size-budget debt in the allowlist.
- Risk / tradeoffs: Low; documentation-only, but drift here directly hurts wake-up clarity and makes cleanup progress hard to trust.
- Suggested validation: `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1`, any doc index/link checks if references move, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Completed: `2026-03-12` - `vendor/radiant` commit `f2d98345` refreshed stale crate-root cleanup comments, and the superproject allowlist plus crate-root metadata now match the actual current oversized-file debt and active cleanup lane.

### 3. [x] Remove duplicated BPM/text-field parsing and pointer-selection logic in `radiant`
- ROI / Effort: High / S-M
- Why it matters: The same BPM parsing and text-hit-selection behavior is implemented in multiple places across shell helpers and the native runtime, which invites silent drift.
- Evidence: Tempo parsing exists in both `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/waveform_toolbar.rs:225` and `vendor/radiant/src/gui_runtime/native_vello/text_bpm.rs:3`; `vendor/radiant/src/gui_runtime/native_vello.rs:693,722,751,786` duplicates browser-search and BPM-field click/index handling in separate paths.
- Recommended change: Extract one shared BPM parser and one shared text-field pointer/index selection helper, then route browser search and waveform BPM editing through the same implementation.
- Risk / tradeoffs: Low-to-moderate; text-edit caret behavior is user-visible, so refactoring should be characterization-driven.
- Suggested validation: Focused parsing tests for empty/invalid/fractional/valid tempo strings, pointer-selection tests for browser search and BPM fields, and `cargo test -p radiant`.
- Completed: `2026-03-12` - `vendor/radiant` commit `2cc5c0f4` adds a shared waveform tempo parser under `src/app`, routes both toolbar/runtime BPM parsing through it, and consolidates browser-search and waveform-BPM pointer-selection handling behind one shared native-vello text-input helper path.

### 4. [ ] Split `vendor/radiant/src/gui/native_shell/state/hit_testing.rs` by interaction surface
- ROI / Effort: High / M
- Why it matters: One 1093-line file currently owns cursor hover classification, source/folder hit testing, context-menu routing, waveform interactions, geometry helpers, and test-only API, so small interaction changes pull broad unrelated churn.
- Evidence: `vendor/radiant/src/gui/native_shell/state/hit_testing.rs:7` starts `handle_cursor_move_effect`; the same file also owns source and folder row hit testing around `:189`, context-menu routing around `:213`, browser row hit testing around `:404`, waveform/button geometry helpers, and cache-key helpers such as `browser_action_hit_test_cache_key` near `:873`.
- Recommended change: Split into cursor-hover classification, sidebar/browser hit testing, waveform interaction routing, and shared geometry/cache-key helpers; move test-only helpers next to the tests that use them.
- Risk / tradeoffs: Moderate; pointer semantics are fragile and regressions here can affect every primary interaction surface.
- Suggested validation: Existing native-shell hit-test tests, targeted hover/action characterization tests, and `cargo test -p radiant native_shell`.

### 5. [ ] Split `vendor/radiant/src/gui_runtime/native_vello/runtime_render.rs` into invalidation, scene, present, and profiling modules
- ROI / Effort: High / L
- Why it matters: The runtime render core is still the highest-risk hotspot in `radiant` because it mixes dirty-state reconciliation, overlay fingerprinting, scene encoding, upload caching, present bookkeeping, and redraw telemetry in one 1074-line file.
- Evidence: `vendor/radiant/src/gui_runtime/native_vello/runtime_render.rs:6` owns `rebuild_scene_if_needed`; `:44` handles invalidation scope application; `:154` emits frame results; `:207` caches image-upload blobs; `:381` calculates static segment revisions; `:485` rebuilds segment scenes; `:552` performs full scene reconciliation; `:847` finalizes redraw/present.
- Recommended change: Keep one thin orchestration layer and extract focused modules for invalidation state, retained scene/cache management, startup reveal/present policy, and redraw profiling/result emission.
- Risk / tradeoffs: High; mistakes here can cause subtle incremental-render or missed-present regressions that are hard to localize.
- Suggested validation: Characterization coverage for overlay fingerprints, scene append ordering, redraw result flags, startup reveal behavior, `cargo test -p radiant`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 6. [ ] Tighten `native_vello` runtime drag/text-input ownership across `native_vello.rs`, `runtime_events.rs`, and `runtime_input.rs`
- ROI / Effort: High / L
- Why it matters: Runtime state for drags, text targets, and pointer lifecycles is spread across multiple files that mutate the same flags, which makes the runtime boundary hard to reason about.
- Evidence: `vendor/radiant/src/gui_runtime/native_vello.rs:143` defines `NativeVelloRunner`; `vendor/radiant/src/gui_runtime/native_vello/runtime_events.rs:61+` and `vendor/radiant/src/gui_runtime/native_vello/runtime_input.rs:113+` both manipulate state such as `volume_drag_active`, `selection_drag_active`, `map_focus_drag_active`, `browser_scrollbar_drag`, `waveform_scrollbar_drag`, and `text_input_target`.
- Recommended change: Consolidate drag/text-input ownership into one explicit state machine module and make event/input modules call through narrow transition helpers rather than mutating runner fields directly.
- Risk / tradeoffs: High; drag lifecycles are timing-sensitive and involve many user-visible edge cases.
- Suggested validation: Pointer press/move/release tests for all drag modes, text-input focus/commit tests, startup/input smoke coverage, and `cargo test -p radiant gui_runtime`.

### 7. [ ] Split `src/app_core/native_bridge/projection_cache/projection_key.rs` into segment-specific key builders
- ROI / Effort: High / M
- Why it matters: Projection-cache invalidation is correctness-sensitive, but one file still builds every status, browser, map, waveform, and non-segment projection key alongside shared hashing/time encoding helpers.
- Evidence: `src/app_core/native_bridge/projection_cache/projection_key.rs:16` builds the root native key; `:124` builds status keys; `:168` and `:188` build browser-frame and browser-rows keys; `:204` builds the map key; `:222` builds the waveform key; `:310` derives waveform timing; `:448` encodes waveform channel view.
- Recommended change: Move each projection-key family into its own module with a tiny shared hashing/encoding helper layer and explicit docs about which controller fields participate in invalidation.
- Risk / tradeoffs: Moderate; any missed field changes cache behavior and can create stale UI projections.
- Suggested validation: `cargo test app_core::native_bridge::tests -- --test-threads=1`, targeted key-drift tests, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 8. [ ] Split `src/app/controller/ui/clipboard_paste/source_job.rs` into explicit prepare, stage, commit, and finalize phases
- ROI / Effort: High / M
- Why it matters: Clipboard source import is a multi-step workflow with filesystem and DB side effects, but it is still concentrated in one file, which makes failure cleanup harder to audit.
- Evidence: `src/app/controller/ui/clipboard_paste/source_job.rs:102` runs the whole job; `:152` prepares paths; `:178` stages copies; `:238` commits DB work; `:285` finalizes; `:307-336` mixes progress reporting and cleanup helpers.
- Recommended change: Split the job into phase-oriented modules plus a small orchestration façade that makes compensation and error-reporting boundaries explicit.
- Risk / tradeoffs: Moderate; rollback ordering and journal cleanup must remain correct on partial failure paths.
- Suggested validation: Existing clipboard/source import tests, targeted failure-path coverage, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 9. [ ] Split `src/app/controller/playback/tagging.rs` into rating mutation, focus advancement, and undo helpers
- ROI / Effort: High / S-M
- Why it matters: Rating changes are user-facing and stateful, yet the current file bundles multi-selection resolution, mutation, filtered-list refocus rules, and undo/redo payload construction in one place.
- Evidence: `src/app/controller/playback/tagging.rs:11` decides auto-advance policy; `:30` implements post-rating focus/commit rules; `:58` computes fallback focus after filtered removal; `:83-211` performs selection traversal, mutation, undo registration, status reporting, and refocus.
- Recommended change: Extract pure focus-policy helpers and undo payload assembly from the imperative controller mutation path so the behavioral branches become easier to test directly.
- Risk / tradeoffs: Low-to-moderate; behavior must remain stable for filtered lists, locked rows, and random-navigation mode.
- Suggested validation: Tagging/filter-navigation tests, undo/redo tests, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 10. [ ] Split `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs` into batch intake, execution, and deferred-update modules
- ROI / Effort: Medium-High / M
- Why it matters: The compute worker still mixes thread lifecycle, queue polling, settings snapshotting, panic capture, decoded-batch grouping, immediate job execution, deferred DB updates, and queue logging in one worker file.
- Evidence: `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs:26-116` runs the worker loop; `:134` snapshots settings; `:156` processes batches; `:187` wraps panic handling and job fan-out; later helpers finalize immediate jobs, flush deferred updates, and log queue state.
- Recommended change: Keep a small worker loop in this file and extract batch planning, job execution, deferred persistence, and logging into dedicated helper modules.
- Risk / tradeoffs: Moderate; concurrency and shutdown semantics must remain unchanged.
- Suggested validation: Analysis job pool tests, worker cancellation/shutdown coverage, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 11. [ ] Split `src/waveform/render/paint/lines.rs` into raster kernels, packing, and pixel-blend helpers
- ROI / Effort: Medium-High / M
- Why it matters: Low-level waveform rasterization is performance-sensitive and visually critical, but one 503-line file still owns mono rendering, stereo packing, interpolation, supersampling, anti-aliased line stepping, and pixel blending without direct local tests.
- Evidence: `src/waveform/render/paint/lines.rs:53,123,178,233,312,500` spans mono rendering, split-stereo packing, interpolation, AA line stepping, and final pixel work; the file has no local `#[cfg(test)]` coverage.
- Recommended change: Extract the raster math and pixel blend primitives into small helpers with targeted unit tests, leaving the public paint entrypoints as thin orchestration.
- Risk / tradeoffs: Moderate; tiny math changes can cause visible waveform regressions or performance drift.
- Suggested validation: Targeted waveform render tests for mono/stereo/AA cases plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 12. [ ] Add focused automation snapshot characterization coverage and split `vendor/radiant/src/gui/native_shell/state/automation.rs` by shell panel family
- ROI / Effort: Medium-High / M
- Why it matters: The automation snapshot is a contract surface for GUI tooling, but the implementation is a 954-line manual builder with large action-ID mapping logic and little direct coverage.
- Evidence: `vendor/radiant/src/gui/native_shell/state/automation.rs:10` builds the full shell snapshot; `:52+` through the rest of the file builds top bar, sidebar, waveform, browser, status, prompt, options, and progress nodes; `:852+` manually maps actions to slugs. Repo-wide searches did not show direct tests for `automation_snapshot`, `capture_gui_automation_snapshot`, or `action_slug`.
- Recommended change: Split snapshot builders by panel family with shared node helpers, and add golden/contract tests for representative browser, map, prompt, progress, and options states.
- Risk / tradeoffs: Moderate; automation IDs are a compatibility surface for tools and should not drift casually.
- Suggested validation: New snapshot contract tests under `vendor/radiant`, targeted `cargo test -p radiant`, and GUI test CLI smoke coverage if automation artifacts are involved.

### 13. [ ] Break `radiant` browser rendering/test blobs into focused list, chrome, truncation, and virtualization modules
- ROI / Effort: Medium-High / M
- Why it matters: Browser UI cleanup currently requires touching broad production and test files that mirror too many concerns at once.
- Evidence: `vendor/radiant/src/gui/native_shell/state/frame_build/browser.rs` is about 686 LOC and mixes row-window rendering, tabs, toolbar/header/footer, list/map switching, metadata chips, and scrollbar paint; `vendor/radiant/src/gui/native_shell/state/browser_rows.rs` is about 781 LOC and mixes layout structs, cache keys, truncation, colors, scrollbar math, and windowing; `vendor/radiant/src/gui/native_shell/state/tests/browser_rows.rs` is about 689 LOC; `vendor/radiant/src/gui/native_shell/mod.rs` also carries an 800+ line omnibus test module after the export surface.
- Recommended change: Separate browser list virtualization, chrome layout, truncation/cache policy, scrollbar math, and scenario-style tests into smaller modules with shared fixtures.
- Risk / tradeoffs: Moderate; browser virtualization and hit-testing are tightly coupled, so characterization tests should move with the extracted helpers.
- Suggested validation: Browser-row truncation/windowing tests, browser toolbar tests, screenshot/contract tests where available, and `cargo test -p radiant native_shell`.

### 14. [ ] Break remaining oversized test hubs into domain-focused modules
- ROI / Effort: Medium / M
- Why it matters: Several large test hubs still mix unrelated behavior families, which makes coverage harder to navigate and encourages future accumulation in the same files.
- Evidence: `src/app_core/native_bridge/tests/projection_cache.rs` is about 609 LOC and mixes key drift, waveform invariants, dirty-segment reuse, overlay refresh, env-flag parsing, and bridge metrics; `src/app_core/controller/tests.rs` is about 459 LOC; `src/app/controller/tests/browser_core.rs` is about 410 LOC; `tests/unit/source_db_mod_tests.rs` is about 496 LOC; `vendor/radiant/src/gui/native_shell/state/tests/browser_rows.rs` is about 689 LOC.
- Recommended change: Split each hub by behavior family with shared fixtures extracted once, so failures point to one concern at a time.
- Risk / tradeoffs: Low; mostly structural, but test names and fixture ergonomics should stay stable.
- Suggested validation: Run the targeted test modules serially where cargo locking matters, then rerun `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 15. [ ] Split `apps/updater-helper/src/ui.rs` into async dataflow, reducer/model, and view helpers
- ROI / Effort: Medium / M
- Why it matters: The updater helper UI is a user-facing workflow surface, but one 517-line file still mixes release fetching, update apply orchestration, log buffering, model projection, and action reduction with thin direct coverage.
- Evidence: `apps/updater-helper/src/ui.rs:59,88,162,193,315,416` spans async release loading, apply orchestration, buffered logging, and reducer/view logic; only two local tests currently live near `:524` and `:543`.
- Recommended change: Separate async release/apply tasks from the reducer/model layer and the actual egui view helpers so UI behavior can be tested without dragging the async plumbing into every edit.
- Risk / tradeoffs: Moderate; UI status transitions and log buffering need to remain stable through async failures.
- Suggested validation: `cargo test -p sempal-updater-helper` and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 16. [ ] Document public action/scenario enums and separate catalog metadata from identifier declarations
- ROI / Effort: Medium / S-M
- Why it matters: The crate denies missing docs at the boundary, but some public enums still opt out, and `catalog.rs` couples stable identifiers, declaration ordering, and metadata lookup in one place.
- Evidence: `src/app_core/actions/catalog.rs:7,232,247,259` suppresses missing docs on public enums and contains a 106-entry `GuiActionKind::ALL`; `src/gui_test/scenario.rs:18,31` suppresses docs for the public `GuiScenarioStep` and `GuiAssertion` enums.
- Recommended change: Add real docs for public variants, separate stable action identifiers from metadata tables/lookup helpers, and keep the scenario assertion contract explicitly documented.
- Risk / tradeoffs: Low; mostly documentation and mechanical structure, but the action ordering contract must remain stable.
- Suggested validation: `cargo test` for any action-catalog or GUI scenario consumers, plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 17. [ ] Split `src/app/controller/playback/audio_options.rs` into pure normalization/probing helpers and controller-side mutation
- ROI / Effort: Medium / M
- Why it matters: The file already contains clean pure logic, but it is still mixed with device probing, config persistence, UI projection, and player rebuild side effects, which makes stateful branches harder to test directly.
- Evidence: `src/app/controller/playback/audio_options.rs:14,94,148,243,350,439` spans normalization, device/rate probing, channel normalization, config persistence, and player rebuild side effects; direct tests currently cover only the pure helper in `src/app/controller/playback/audio_options_tests.rs:20,40,60`.
- Recommended change: Move pure normalization and probing policy into small helpers with their own tests, and leave the controller methods responsible only for wiring settings, UI state, and player updates together.
- Risk / tradeoffs: Low-to-moderate; device selection and fallback behavior must remain unchanged across platforms.
- Suggested validation: Targeted audio-option tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 18. [ ] Split `src/waveform/zoom_cache.rs` into cache-core logic and telemetry instrumentation
- ROI / Effort: Medium / M
- Why it matters: The zoom cache mixes sharded cache storage, eviction, poison recovery, resident-byte accounting, and hot-path telemetry globals, which complicates both reasoning and targeted tests.
- Evidence: `src/waveform/zoom_cache.rs:16-37` defines the cache plus telemetry globals; `:38-152` is telemetry accounting/log emission; `:165+` starts the cache API and lock management. The file is about 489 LOC.
- Recommended change: Keep the cache data structure and eviction policy in one module, move telemetry counters/logging into a small instrumentation helper, and add focused tests around eviction/poison-recovery behavior.
- Risk / tradeoffs: Moderate; concurrency and telemetry overhead should remain unchanged.
- Suggested validation: Existing waveform cache/render tests, any hot-path telemetry smoke checks, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 19. [ ] Add direct hotkey-registry coverage and narrow the static registry surface
- ROI / Effort: Medium / S-M
- Why it matters: The hotkey registry is a large user-facing data table, but it has very little direct coverage, so collisions, duplicate gestures, and scope drift can slip through quietly.
- Evidence: `src/app/controller/ui/hotkeys/actions.rs` is about 454 LOC of static registry data with key clusters around `:5`, `:175`, `:211`, and `:260`; only indirect coverage was found in controller tests such as `src/app/controller/tests/focus_random.rs:208`.
- Recommended change: Extract smaller declarative registry chunks or helper constructors where useful, and add direct tests for duplicate gestures, scope invariants, and representative action bindings.
- Risk / tradeoffs: Low; mostly declarative cleanup, but the registry is user-visible so accidental keymap drift must be caught.
- Suggested validation: New hotkey-focused unit tests plus existing controller hotkey coverage and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
