# Cleanup Plan (ROI Ranked)

Generated: 2026-03-11 (UTC)
Phase: Phase 2 in progress; items 1-6 complete, item 7 next
Status legend: `[ ]` pending, `[x]` done
Project language/tooling: Rust 2024 Cargo workspace (`sempal` + `apps/*` + `tools/*` + `vendor/radiant`)
Canonical local CI command: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Audit Notes

- The previous 30-item cleanup backlog is complete; this refresh only includes new debt that remains on the current `next` head.
- The highest current ROI is concentrated in remaining multi-responsibility runtime/projection modules, waveform infrastructure that still mixes model/render/IO concerns, and oversized benchmark/test harnesses that obscure coverage boundaries.
- Public API docs are still broadly enforced, so the main cleanup debt is structural: ownership clarity, reusable helpers, and better seams for tests and future changes.
- The backlog intentionally front-loads smaller locally owned refactors before the largest `vendor/radiant` runtime decompositions.

## Ordered Backlog

- [x] 1) Split native-bridge metrics into registry, snapshot, and reporting modules
  - ROI/Effort: High / M
  - Why it matters: bridge profiling changes still require editing one large file that mixes process-lifetime counters, snapshot capture, env-flag policy, and formatted reporting. That raises drift risk in one of the main perf-observability surfaces.
  - Evidence:
    - `src/app_core/native_bridge/metrics.rs` is 863 LOC.
    - `BridgeMetrics` counter registry is defined at `src/app_core/native_bridge/metrics.rs:39`.
    - `BridgeMetricsSnapshot` capture/derivation starts at `src/app_core/native_bridge/metrics.rs:186`.
    - The string/report formatting path starts at `src/app_core/native_bridge/metrics.rs:398`.
  - Recommended change: split the file into focused modules for counter storage, snapshot capture/aggregation, and log/report formatting while keeping the existing feature-gated API stable.
  - Risk/tradeoffs: Medium. Profiling behavior must stay byte-for-byte compatible enough for existing perf tooling and log parsing.
  - Suggested validation: native-bridge metrics tests, targeted perf/projection tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-11 (`0286445a`)

- [x] 2) Finish splitting app-core native-shell projection assembly
  - ROI/Effort: High / M
  - Why it matters: app-core native-shell projection still has one root file that mixes derived-input gathering, core model materialization, overlay/chrome assembly, and cache instrumentation. It remains a central seam for every native frame pull.
  - Evidence:
    - `src/app_core/native_shell.rs` is 451 LOC.
    - Derived/core/chrome assembly structs are clustered at `src/app_core/native_shell.rs:159`, `:175`, and `:191`.
    - Projection pipeline steps are still concentrated in `derive_project_app_model_inputs` (`:209`), `materialize_project_app_model_core` (`:225`), `materialize_project_app_model_overlay_and_chrome` (`:240`), and `assemble_project_app_model` (`:256`).
    - Tests still live in the same root file starting at `src/app_core/native_shell.rs:472`.
  - Recommended change: move derivation, section materialization, and assembly into a small `native_shell/` module tree with the root file acting only as a documented facade.
  - Risk/tradeoffs: Medium. Projection payload shapes and retained-cache hooks must remain stable for native bridge callers.
  - Suggested validation: `app_core::native_shell` tests, native-bridge projection tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-11 (`517ec252`)

- [x] 3) Decompose recording waveform loading into IO, retained-state, and incremental-update modules
  - ROI/Effort: High / M
  - Why it matters: recording-waveform refreshes are still driven by one top-level worker module that mixes filesystem reads, retained incremental state, decode fallback policy, and test helpers. This area is performance-sensitive and still awkward to extend safely.
  - Evidence:
    - `src/app/controller/playback/recording/waveform_loader.rs` is 527 LOC.
    - The main worker path remains one large `load_recording_waveform(...)` function from `src/app/controller/playback/recording/waveform_loader.rs:49` through `:281`.
    - The function directly handles metadata checks, cache-key/state eviction, full-file first load, incremental rebuild fallback, and state-map reinsertion.
    - Tests still live in the same file from `src/app/controller/playback/recording/waveform_loader.rs:284`.
  - Recommended change: keep the existing `aggregation`, `decode`, `queue`, and `result` submodules, but move the remaining top-level load/state-flow into dedicated `io`, `state_cache`, and `incremental_update` modules with tests beside those seams.
  - Risk/tradeoffs: Medium. Request-id gating, truncation handling, and no-change behavior must remain identical.
  - Suggested validation: recording waveform loader tests, playback recording tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-11 (`0524d980`)

- [x] 4) Split the waveform public surface into model, loading, and render facades
  - ROI/Effort: High / M
  - Why it matters: the top-level waveform module still mixes image/pixel types, decoded-audio state, span-analysis helpers, renderer construction, and file IO limits. That keeps the core waveform contract harder to navigate than the rest of the already-split codebase.
  - Evidence:
    - `src/waveform/mod.rs` is 472 LOC.
    - Public waveform data types live at `src/waveform/mod.rs:18`, `:61`, `:77`, `:88`, and `:129`.
    - Span-analysis helpers live in the same file at `src/waveform/mod.rs:144-231`.
    - Renderer + disk-read responsibilities are still in the same file at `src/waveform/mod.rs:422-464`.
    - `src/waveform/render.rs` is another 542 LOC and still mixes viewport orchestration, cache usage, visible-slice fallback, and fade-preview rendering.
  - Recommended change: move data-model types, decode/load entrypoints, and renderer facade responsibilities into a clearer module tree so `mod.rs` becomes a thin documented re-export surface.
  - Risk/tradeoffs: Medium. The public waveform API is used widely, so import churn needs to stay contained behind re-exports.
  - Suggested validation: waveform unit tests, controller waveform tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-11 (`5fea7bc1`)

- [x] 5) Split drag-drop controller actions by payload family and drop-target resolution
  - ROI/Effort: High / M
  - Why it matters: drag/drop behavior is still concentrated in one action file that has to understand samples, folders, selections, browser triage, folder targets, source moves, and drop-target reordering. That makes any new drag rule expensive to reason about.
  - Evidence:
    - `src/app/controller/ui/drag_drop_controller/actions.rs` is 464 LOC.
    - The trait and shared drag entrypoints are defined at `src/app/controller/ui/drag_drop_controller/actions.rs:8-42`.
    - Stateful drag updates live at `src/app/controller/ui/drag_drop_controller/actions.rs:113-163`.
    - Finish/drop resolution begins at `src/app/controller/ui/drag_drop_controller/actions.rs:165` and immediately expands into a wide target/payload dispatch matrix starting around `:187`.
  - Recommended change: separate drag start/update lifecycle from payload-specific finish handlers (samples, folders, selections, drop-target reorder) and isolate target resolution into explicit helpers.
  - Risk/tradeoffs: Medium. Drag behavior is highly user-visible and regression-prone around modifier keys and folder-target fallback.
  - Suggested validation: drag/drop controller tests, folder-move tests, selection export drag tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-11 (`e29d8464`)

- [x] 6) Split the GUI benchmark harness into workspace seeding, scenario registry, and report assembly
  - ROI/Effort: Medium-High / M
  - Why it matters: the bench harness is now large enough that perf-scenario additions require touching setup, run-loop orchestration, and result shaping in the same places. That increases maintenance cost for the repository’s main latency guardrails.
  - Evidence:
    - `tools/bench-cli/src/bench/gui.rs` is 357 LOC.
    - `run(...)` concentrates workspace creation, all scenario execution, and result assembly at `tools/bench-cli/src/bench/gui.rs:112-220`.
    - Workspace/data seeding helpers remain in the same file at `tools/bench-cli/src/bench/gui.rs:255-347`.
    - `tools/bench-cli/src/bench/gui/interactions.rs` is 366 LOC and repeats the same staged benchmark wrapper shape across many scenarios starting at `:28`, `:53`, `:83`, `:108`, `:133`, `:158`, and `:183`.
  - Recommended change: extract a small scenario registry/shared staged-run helper layer, keep workspace seeding isolated, and leave the top-level benchmark entrypoint as a short orchestration facade.
  - Risk/tradeoffs: Medium. Benchmark semantics and JSON output must stay stable so perf guard thresholds remain comparable.
  - Suggested validation: bench-cli tests, benchmark JSON smoke run, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
  - Completion: 2026-03-11 (`d702d343`, `4cc2ad7e`)

- [ ] 7) Split audio source combinators into focused modules with shared sample-accounting helpers
  - ROI/Effort: Medium-High / M
  - Why it matters: the core `Source` trait still lives beside all concrete combinators, buffering behavior, and fade helpers. That keeps low-level playback primitives harder to review and evolve independently.
  - Evidence:
    - `src/audio/source.rs` is 469 LOC.
    - The `Source` trait plus combinator constructors occupy the start of the file at `src/audio/source.rs:7-116`.
    - Concrete wrappers then continue in one file: `SamplesBuffer` (`:141`), `TakeDuration` (`:207`), `TakeSamples` (`:257`), `RepeatInfinite` (`:307`), `Buffered` (`:354`), and `FadeIn` (`:419`).
  - Recommended change: separate the trait/facade from the concrete wrapper implementations and share duration/sample accounting utilities instead of repeating that policy inside one large file.
  - Risk/tradeoffs: Medium. These primitives sit on playback hot paths and must remain allocation-light and behaviorally identical.
  - Suggested validation: audio source tests, playback transport tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 8) Separate normalization dispatch from SIMD backends and shared scalar math
  - ROI/Effort: Medium / S-M
  - Why it matters: normalization still repeats the same runtime CPU-feature dispatch and scalar fallback shape across peak, peak-limit, and RMS flows. That duplication makes SIMD changes noisy and raises consistency risk.
  - Evidence:
    - `src/analysis/audio/normalize.rs` is 504 LOC.
    - Top-level entrypoints repeat dispatch logic at `src/analysis/audio/normalize.rs:7`, `:47`, and `:87`.
    - Shared scalar helpers live later in the same file at `src/analysis/audio/normalize.rs:150-171`.
    - AVX2 and SSE2 backends start at `src/analysis/audio/normalize.rs:173` and `:210`, with similar chunking/reduction structure.
  - Recommended change: keep the current behavior but isolate platform dispatch, scalar reference math, and SIMD backend implementations into separate helpers/modules.
  - Risk/tradeoffs: Low-medium. DSP behavior must remain numerically stable, and refactors must not introduce extra allocations.
  - Suggested validation: normalize/audio analysis tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 9) Break the remaining controller test hubs into behavior-focused modules
  - ROI/Effort: Medium / M
  - Why it matters: several large controller test files are still acting as catch-all behavior bins, which slows navigation and hides gaps by feature domain.
  - Evidence:
    - `src/app/controller/tests/browser_actions.rs` is 672 LOC and spans hotkeys, preview focus, scrolling, delete, normalize, export, and rating flows.
    - `src/app/controller/tests/folders_core.rs` is 577 LOC and mixes creation, rename, delete recovery, selection/filter semantics, and focus rules.
    - `src/app/controller/tests/waveform.rs` is 498 LOC and mixes load/reset behavior, destructive edits, hotkeys, and playback follow-up.
  - Recommended change: split these hubs into smaller modules that mirror production seams (browser focus/navigation, browser destructive actions, folder CRUD/recovery, waveform edit commands, waveform playback interactions).
  - Risk/tradeoffs: Low-medium. Mostly structural, but inconsistent naming could make tests harder to find if the split is sloppy.
  - Suggested validation: affected controller test modules, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 10) Factor analysis-jobs DB tests around a shared fixture/schema builder
  - ROI/Effort: Medium / S
  - Why it matters: the analysis-jobs DB tests are large, repeat a hand-built schema, and manually insert rows over and over. That increases maintenance cost for persistence changes and makes the test intent noisier than necessary.
  - Evidence:
    - `src/app/controller/library/analysis_jobs/db/tests.rs` is 638 LOC.
    - `conn_with_schema()` builds the full schema inline at `src/app/controller/library/analysis_jobs/db/tests.rs:4-67`.
    - Many tests repeat direct `INSERT` setup patterns starting at `:72`, `:88`, `:113`, `:134`, `:188`, and `:208`.
  - Recommended change: extract a tiny fixture DSL/shared setup helpers for schema + common sample/job rows while preserving the current test coverage and determinism.
  - Risk/tradeoffs: Low. This is test-only cleanup, but the helper layer should stay small enough that tests remain readable.
  - Suggested validation: analysis-jobs DB tests, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 11) Split waveform overlay rendering in `radiant` by scrollbar, selection, fades, and trail
  - ROI/Effort: Medium-High / M
  - Why it matters: waveform overlay rendering is now one of the densest feature areas in `radiant`, and recent selection/fade/playhead changes continue to stack into one file.
  - Evidence:
    - `vendor/radiant/src/gui/native_shell/state/waveform_segments.rs` is 1161 LOC.
    - Scrollbar rendering starts at `vendor/radiant/src/gui/native_shell/state/waveform_segments.rs:262`.
    - Selection/resize edge helpers cluster through `:366-569`.
    - Edit fade overlays and handle geometry begin at `:626`, with more handle helpers at `:796-971`.
    - Playhead trail rendering starts at `:983`.
  - Recommended change: split the file into focused sibling modules for scrollbar/view-window chrome, selection handles, fade overlays, and playhead trail rendering.
  - Risk/tradeoffs: Medium. Overlay geometry is visually sensitive, so the split needs strong targeted tests.
  - Suggested validation: targeted `radiant` waveform overlay tests run serially, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 12) Split `radiant` static frame building into browser, map, waveform, and chrome builders
  - ROI/Effort: High / L
  - Why it matters: the static frame builder still owns the bulk of the native-shell painting contract, so any visual change requires navigating one very large mixed-responsibility function and many trailing helpers.
  - Evidence:
    - `vendor/radiant/src/gui/native_shell/state/frame_build.rs` is 2390 LOC.
    - The core `build_frame_with_style_into_with_motion_sinks(...)` implementation starts at `vendor/radiant/src/gui/native_shell/state/frame_build.rs:9` and immediately mixes global chrome fills, waveform overlay, browser rows/map panel, and toolbar rendering in one path.
    - Large trailing helper groups remain in the same file for BPM grid (`:2101`), browser row borders (`:2159`), context menus (`:2208`), and waveform toolbar hover hints (`:2276`).
  - Recommended change: extract browser/table, map, waveform, overlay, and chrome/status builders into explicit modules and leave `frame_build.rs` as a short orchestration shell.
  - Risk/tradeoffs: High. Paint ordering and retained dirty-segment behavior must remain exact.
  - Suggested validation: targeted `radiant` native-shell tests run serially, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 13) Split `radiant` native-shell state into caches, hit-testing, and toolbar/browser helpers
  - ROI/Effort: High / L
  - Why it matters: `NativeShellState` still owns mutable UI state, truncation caches, toolbar hit-test caches, hover logic, browser helper geometry, map hit-testing, and waveform toolbar rendering helpers in one file.
  - Evidence:
    - `vendor/radiant/src/gui/native_shell/state.rs` is 3533 LOC.
    - State/caches are defined at `vendor/radiant/src/gui/native_shell/state.rs:91-137` and `:139-245`.
    - The main `impl NativeShellState` starts at `vendor/radiant/src/gui/native_shell/state.rs:669`.
    - Free helper functions for hit-testing and toolbar/browser geometry dominate the later half from roughly `vendor/radiant/src/gui/native_shell/state.rs:2286-3642`.
  - Recommended change: continue the previous state split by extracting cache types, hover/hit-testing, browser toolbar geometry, and waveform toolbar rendering helpers into clearly named sibling modules.
  - Risk/tradeoffs: High. This is central interaction state; subtle hit-test or hover regressions are easy to introduce.
  - Suggested validation: targeted `radiant` native-shell state tests run serially, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 14) Further decompose `radiant` native Vello runtime into runner, profiler, caches, and text/BPM helpers
  - ROI/Effort: High / L
  - Why it matters: the native runtime entrypoint is still the largest Rust file in the repo and continues to mix event-loop state, frame caches, startup profiling, scene fingerprints, helper parsing, and public runners.
  - Evidence:
    - `vendor/radiant/src/gui_runtime/native_vello.rs` is 4186 LOC.
    - Immediate drag/profile/runtime state types cluster at `vendor/radiant/src/gui_runtime/native_vello.rs:83-271`.
    - Cache fingerprinting and static-segment scene cache structures still live together at `:643-1097`.
    - The main runner state begins at `vendor/radiant/src/gui_runtime/native_vello.rs:1299`.
    - BPM/text helper parsing remains near the end at `:3678-3732`, with public runners at `:4336-4391`.
  - Recommended change: keep the public runtime facade stable but split runner state, profiling/startup timing, static-segment caches/fingerprints, and misc helper parsing into focused modules.
  - Risk/tradeoffs: High. This is hot-path event-loop/render code and one of the highest-regression-risk areas in the repo.
  - Suggested validation: targeted `radiant` runtime/input tests run serially, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, then `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

- [ ] 15) Deduplicate installer and updater-helper native GUI bridges
  - ROI/Effort: Low-Medium / M
  - Why it matters: the small companion apps now each carry their own large `radiant` bridge UI file with similar progress/status plumbing, icon decoding, and process-exit glue. That duplication will keep drifting as these UIs evolve.
  - Evidence:
    - `apps/updater-helper/src/ui.rs` is 530 LOC, with bridge/runtime state starting at `:64`, `NativeAppBridge` at `:430`, and local process-exit helpers at `:486`.
    - `apps/installer/src/ui.rs` is 461 LOC, with comparable bridge/runtime state starting at `:48`, `NativeAppBridge` at `:352`, and local icon/process helpers at `:411-452`.
  - Recommended change: extract a small shared native-app helper layer or at least align each app into the same internal module shape (view state, bridge adapter, icon/process helpers) so future maintenance is symmetrical.
  - Risk/tradeoffs: Medium. These binaries are small, so over-abstracting would hurt clarity; the change should stay intentionally lightweight.
  - Suggested validation: app-specific unit tests or smoke builds plus `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.

## Progress Log

- 2026-03-11: Read repository guidance first (`AGENTS.md`, `README.md`, `docs/README.md`, `docs/plans/index.md`, `docs/plans/active/runtime_performance_exec_plan.md`, `docs/plans/active/todo.md`, and `MEMORY.md`) before refreshing this plan.
- 2026-03-11: Confirmed the canonical local CI parity command for the current Windows environment is `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
- 2026-03-11: Verified the previous cleanup backlog is fully complete on the current `next` head and generated this second-pass backlog only from remaining current hotspots.
- 2026-03-11: Audit evidence came from targeted scans of `src/app_core`, `src/waveform`, `src/audio`, `src/analysis`, `src/app/controller`, `tools/bench-cli`, `apps/*`, and `vendor/radiant/src`, plus current file-size and suppression searches.
- 2026-03-11: Phase 2 resumed after explicit user confirmation.
- 2026-03-11: Completed item 1 in commit `0286445a` by splitting native-bridge metrics into focused registry, snapshot, and reporting modules while keeping the trace-hook facade stable.
- 2026-03-11: Completed item 2 in commit `517ec252` by moving the staged top-level app-model projection pipeline into `src/app_core/native_shell/app_model.rs` and leaving `src/app_core/native_shell.rs` as a thinner facade.
- 2026-03-11: Completed item 3 in commit `0524d980` by splitting recording waveform loading into focused `io`, `state_cache`, `incremental_update`, and test modules while preserving incremental refresh behavior.
- 2026-03-11: Completed item 4 in commit `5fea7bc1` by turning `src/waveform/mod.rs` into a thin facade and moving the waveform public model and load-from-disk entrypoints into focused `model` and `loading` modules.
- 2026-03-11: Completed item 5 in commit `e29d8464` by splitting drag/drop action handling into focused drop-target resolution, payload-specific finish handlers, and external-drag timing modules.
- 2026-03-11: Completed item 6 in commits `d702d343` and `4cc2ad7e` by splitting the GUI benchmark harness into focused workspace seeding, scenario registry, and report assembly modules, then aligning the benchmark interaction helpers with the current waveform selection action contract.
