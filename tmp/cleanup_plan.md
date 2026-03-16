# Cleanup Audit Backlog

- Refreshed (UTC): `2026-03-12T16:32:35Z`
- Sempal branch/head at audit time: `next` / `34ff29b2`
- Radiant branch/head at audit time: `next` / `711f159a`
- Phase: `Phase 1 complete`
- Status: `Parked; resume only after explicit user confirmation before Phase 2 implementation`
- Note: later GUI/browser AIV work landed on `next`; this file remains the last cleanup audit snapshot, not a refreshed post-AIV audit.
- Historical-only warning: some backlog items below refer to paths or module names that have since been split, removed, or relocated. Treat this file as a dated planning snapshot, not as a live file map; use `tmp/cleanup_audit_hotspots.md` plus the active improvement audit for current-tree prioritization.
- Maintenance note (2026-03-16): live debt-tracking inputs were refreshed again in the active improvement-audit lane; the current allowlist is now empty, `tmp/cleanup_audit_hotspots.md` is the live hotspot snapshot, and this parked backlog remains historical context rather than a live file map.
- Canonical dev loop (Windows): `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Canonical quick gate (Windows): `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Canonical full local CI (Windows): `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Ordered ROI backlog

### 1. [ ] Split `vendor/radiant/src/gui/native_shell/state/automation.rs` by panel family and add direct contract tests
- ROI / Effort: High / M
- Why it matters: the GUI automation snapshot is now a contract surface for the CLI runner, scenario packs, and AIV automation, but one 1005-line builder still owns the whole shell snapshot plus the stable action-id mapping.
- Evidence: `vendor/radiant/src/gui/native_shell/state/automation.rs` builds sources, waveform, browser, map, status, options, prompt, progress, and update nodes in one file; `action_slug` is a single large `UiAction` mapping near the end of the file; the public wrapper in `src/gui_runtime/mod.rs` exposes this contract, but current tests mainly cover root-level runner behavior rather than panel-by-panel snapshot shape.
- Recommended change: split snapshot builders into focused panel modules with shared node helpers, isolate the action-id mapping behind one documented helper, and add characterization tests for representative browser, map, prompt, progress, and update states.
- Risk / tradeoffs: Moderate; automation node ids and action ids are compatibility surfaces and must not drift accidentally.
- Suggested validation: `cargo test -p radiant -- --test-threads=1`, `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 2. [ ] Split `src/app/controller/playback/tagging.rs` into rating mutation, focus policy, and undo helpers
- ROI / Effort: High / S-M
- Why it matters: rating changes are user-facing and stateful, but the current file interleaves selection resolution, mutation, filtered-list refocus rules, auto-trash behavior, and duplicate undo closure assembly.
- Evidence: `src/app/controller/playback/tagging.rs` uses `should_advance_after_rating`, `advance_or_commit_after_rating`, and `next_focus_path_for_removed_rows` to feed two long imperative paths, `tag_selected` and `adjust_selected_rating`, which both rebuild row contexts and construct near-duplicate undo logic.
- Recommended change: extract pure focus-policy helpers, row-context collection, and undo payload assembly into small helpers or modules, leaving one thin controller mutation path per command.
- Risk / tradeoffs: Low-to-moderate; filtered removal, locked rows, random-navigation mode, and auto-trash semantics must stay stable.
- Suggested validation: targeted tagging and refocus tests in the browser controller suite, undo/redo coverage, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 3. [ ] Split `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs` into loop, execution, and deferred-finalization modules
- ROI / Effort: High / M
- Why it matters: the analysis compute worker is a concurrency hotspot, and one file still owns shutdown polling, queue waits, settings snapshots, allow-list checks, panic capture, decoded-batch grouping, immediate job execution, deferred DB updates, and queue logging.
- Evidence: `run_compute_worker` handles the main loop and repaint cadence; `process_batch`, `process_batch_work`, `run_work_item`, and `handle_analysis_work` mix intake and execution decisions; `finalize_immediate_jobs` and `flush_deferred_updates` own persistence completion.
- Recommended change: keep a small worker loop in `compute_worker.rs`, move batch planning and execution into focused helpers, and isolate deferred-finalization/logging into dedicated modules with clearer ownership.
- Risk / tradeoffs: Moderate; shutdown, inflight-queue cleanup, and deferred-update ordering are timing-sensitive.
- Suggested validation: targeted analysis job pool tests, worker cancellation/shutdown coverage, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 4. [ ] Break `radiant` browser shell rendering/state/test blobs into focused list, chrome, truncation, and virtualization modules
- ROI / Effort: High / M
- Why it matters: browser-shell changes still require editing broad production and test blobs that each mix too many concerns, which slows down cleanup and raises regression risk around virtualization and hit-testing.
- Evidence: `vendor/radiant/src/gui/native_shell/state/browser_rows.rs` mixes cache keys, truncation, layout, scrollbar math, and row-windowing; `vendor/radiant/src/gui/native_shell/state/frame_build/browser.rs` mixes row painting, tabs, toolbar, footer, and list/map chrome; `vendor/radiant/src/gui/native_shell/state/tests/browser_rows.rs` and `vendor/radiant/src/gui/native_shell/mod.rs` remain oversized omnibus test hubs.
- Recommended change: separate list virtualization, truncation/cache policy, scrollbar math, toolbar/footer chrome, and scenario-style tests into smaller modules with shared fixtures.
- Risk / tradeoffs: Moderate; browser row windowing, truncation, and hit-target geometry are tightly coupled and need characterization coverage to move safely.
- Suggested validation: `cargo test -p radiant -- --test-threads=1`, native-shell browser-focused tests, shot tests where affected, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 5. [ ] Continue decomposing `vendor/radiant` native Vello runtime around runner state, event routing, and scene rebuild ownership
- ROI / Effort: High / L
- Why it matters: recent cleanup reduced some surface area, but the native Vello runtime still centers ownership on a large runner and a few broad control-flow hubs, which keeps perf and correctness changes risky.
- Evidence: `vendor/radiant/src/gui_runtime/native_vello.rs` still holds a large runner state object; `runtime_startup.rs` builds that state in one long constructor; `runtime_events.rs` still owns a broad event-routing match; `runtime_render/scene.rs` remains a large scene-rebuild hub that mixes model pulls, dirty-segment resolution, overlay rebuilds, composition, and profiling.
- Recommended change: keep the public runtime API stable while extracting smaller ownership modules for runner state initialization, event routing transitions, and scene rebuild orchestration.
- Risk / tradeoffs: High; input, startup, and redraw semantics are timing-sensitive and already performance-tuned.
- Suggested validation: targeted `radiant` runtime tests, pointer/startup coverage, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 6. [ ] Split `src/waveform/render/paint/lines.rs` into raster math, AA stepping, and pixel blend helpers with direct unit tests
- ROI / Effort: Medium-High / M
- Why it matters: waveform line rendering is performance-sensitive and visually critical, but one file still owns interpolation, supersampling, split-stereo packing, alpha blending, and anti-aliased line stepping without local tests.
- Evidence: `paint_line_image` and `paint_split_line_image` orchestrate rendering; `sample_at_frame`, `supersampled_frame`, and `catmull_rom` own interpolation; `blend_pixel`, `draw_line_aa`, and `plot_aa` own low-level raster math; the file currently ends without `#[cfg(test)]` coverage.
- Recommended change: extract reusable raster kernels and blend helpers into small focused modules, keep the public paint entrypoints thin, and add deterministic tests for interpolation edges, steep and vertical lines, split-gap packing, and blend output.
- Risk / tradeoffs: Moderate; tiny math differences can create visible waveform regressions or performance drift.
- Suggested validation: waveform render unit tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 7. [ ] Replace public-doc suppressions on GUI contract enums and separate catalog metadata from identifier declarations
- ROI / Effort: Medium-High / S-M
- Why it matters: the repository denies missing docs, but stable GUI contract surfaces still opt out right where external tools need semantics and stability expectations to be explicit.
- Evidence: `src/app_core/actions/catalog.rs` suppresses docs on `GuiActionKind`, `GuiSurface`, `GuiEffectClass`, and `GuiCoverageLayer`; `src/gui_test/scenario.rs` suppresses docs on `GuiScenarioStep` and `GuiAssertion`; the catalog file also couples stable identifiers, metadata, and lookup behavior in one place.
- Recommended change: add real variant-level docs, separate stable identifiers from metadata lookup tables where practical, and add table-driven tests that exercise every scenario step and assertion variant directly.
- Risk / tradeoffs: Low; mostly documentation and structural cleanup, but stable action ordering and ids must remain unchanged.
- Suggested validation: targeted action-catalog and GUI scenario tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 8. [ ] Add direct hotkey-registry invariants and narrow the `HOTKEY_ACTIONS` table surface
- ROI / Effort: Medium / S-M
- Why it matters: the hotkey registry is a large user-facing data table, and today it relies mostly on indirect controller tests rather than direct duplicate-id, duplicate-gesture, and scope-invariant checks.
- Evidence: `src/app/controller/ui/hotkeys/actions.rs` is a 455-line static table covering global, browser, folder, and waveform bindings; `src/app/controller/ui/hotkeys/mod.rs` is thin; current coverage is mostly indirect through controller behavior tests.
- Recommended change: split the registry into smaller declarative chunks if that improves readability, and add direct tests for duplicate ids, duplicate gestures within a scope, and representative lookup behavior.
- Risk / tradeoffs: Low; the main risk is accidental keymap drift while reorganizing the table.
- Suggested validation: hotkey-focused unit tests, existing hotkey controller coverage, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 9. [ ] Split `apps/updater-helper/src/ui.rs` into async tasks, reducer/model, and view helpers
- ROI / Effort: Medium / M
- Why it matters: the updater helper is a user-facing workflow, but one file still mixes release fetching, update apply orchestration, log buffering, model projection, and action reduction with only light direct coverage.
- Evidence: `apps/updater-helper/src/ui.rs` owns async release loading, apply orchestration, background polling, app-model projection, and reducer logic in one module, while only a small local test block exists at the end.
- Recommended change: move background release/apply tasks out of the reducer/model layer, keep the state machine explicit, and isolate view-projection helpers so UI behavior can be tested without async plumbing in every edit.
- Risk / tradeoffs: Moderate; status transitions and buffered logging need to remain stable through async failures.
- Suggested validation: `cargo test -p updater-helper`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 10. [ ] Separate pure audio-option normalization/probing policy from controller-side mutation in `audio_options.rs`
- ROI / Effort: Medium / M
- Why it matters: the file already contains useful pure logic, but it is still mixed with device probing, config persistence, UI projection, and player rebuild side effects, which makes stateful branches harder to test directly.
- Evidence: `src/app/controller/playback/audio_options.rs` combines `normalize_audio_options` with refresh/probe paths, persistence, player rebuilds, and fallback-message formatting; only the pure normalization helper currently has dedicated tests.
- Recommended change: move pure normalization and probing policy into focused helpers with their own tests, leaving controller methods responsible only for wiring state, config, and player updates together.
- Risk / tradeoffs: Low-to-moderate; device selection and fallback behavior must remain unchanged across platforms.
- Suggested validation: targeted audio-options tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 11. [-] Historical item: `vendor/radiant/src/gui/native_shell/shots.rs` was already split in a later lane
- ROI / Effort: Medium / M
- Why it matters: the historical cleanup snapshot still referenced a file that no longer exists in the live tree.
- Evidence: `vendor/radiant/src/gui/native_shell/shots.rs` is absent from the current repository because that work already landed in a later lane.
- Recommended change: do not reopen this item from the parked cleanup plan; rely on the active improvement audit and the refreshed hotspot snapshot for current-tree prioritization.
- Risk / tradeoffs: None beyond losing an outdated historical placeholder.
- Suggested validation: confirm the file remains absent and keep `tmp/cleanup_audit_hotspots.md` current.

### 12. [ ] Split `src/waveform/zoom_cache.rs` into cache-core logic and telemetry instrumentation
- ROI / Effort: Medium / M
- Why it matters: the zoom cache mixes sharded cache storage, poison recovery, eviction, and resident-byte accounting with hot-path telemetry globals and log emission, which makes the low-level behavior harder to reason about.
- Evidence: the top of `src/waveform/zoom_cache.rs` is mostly telemetry flags, counters, and emission helpers, while `WaveformZoomCache`, `lock_shard`, and `CacheInner` own the actual cache policy and locking behavior further down the file.
- Recommended change: keep the cache data structure and eviction policy in one focused module, move telemetry counters/logging into a small instrumentation helper, and add a few targeted resident-byte and eviction assertions where useful.
- Risk / tradeoffs: Moderate; concurrency behavior and telemetry overhead must remain stable.
- Suggested validation: existing zoom-cache tests, any added resident-byte or eviction tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

### 13. [ ] Break remaining oversized test hubs into domain-focused modules
- ROI / Effort: Medium / M
- Why it matters: several large test hubs still mix unrelated behavior families, which makes failures harder to localize and encourages future accumulation in the same files.
- Evidence: `src/app_core/native_bridge/tests/projection_cache.rs` mixes full-key drift, retained-segment dirty-mask behavior, env-flag parsing, and metrics assertions; `src/app/controller/tests/browser_core.rs` mixes source-state, browser indexing/filtering, tagging/refocus, and blur-selection behavior; `vendor/radiant/src/gui/native_shell/mod.rs` still holds a large omnibus native-shell test module.
- Recommended change: split each test hub by behavior family, extract shared fixtures once, and keep test module names aligned with the production ownership boundaries they protect.
- Risk / tradeoffs: Low; mostly structural, but fixture ergonomics and targeted command filters should stay easy to use.
- Suggested validation: run the split test modules serially where cargo locking matters, then rerun `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
