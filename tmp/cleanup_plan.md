# Cleanup Audit Backlog

- Refreshed (UTC): `2026-03-11T17:58:33Z`
- Branch: `next`
- Head: `17d911b2`
- Phase: `Phase 2 in progress`
- Status: `Items 1-7 complete; continuing strict sequential implementation at item 8`
- Canonical quick gate (Windows): `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Canonical full local CI (Windows): `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Ordered ROI backlog

### 1. [x] Split `vendor/radiant/src/gui_runtime/native_vello.rs` runner/event/render monolith
- ROI / Effort: High / L
- Why it matters: The native Vello runtime is the single largest live hotspot in the repo and still concentrates window lifecycle, event dispatch, scene rebuild orchestration, presentation, caches, and startup behavior in one file.
- Evidence: `vendor/radiant/src/gui_runtime/native_vello.rs` is about 2936 LOC. `NativeVelloRunner` starts near line 140, its main impl starts near line 265, and redraw/present/event-lifecycle code is still intertwined in one façade despite recent helper extraction.
- Recommended change: Split the file into focused modules for window lifecycle, event dispatch, frame/rebuild scheduling, render/present, and retained cache/resource management while keeping the public runtime entrypoints stable.
- Risk / tradeoffs: High churn across lifetime-heavy types and `winit` callback wiring; careless moves could regress startup timing or pointer/keyboard behavior.
- Suggested validation: Targeted `radiant` runtime/input tests, startup smoke tests, `cargo fmt --all`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
- Completed: `2026-03-11` — `vendor/radiant` commit `e0ce0710` split startup/window lifecycle, scene rebuild/present, and event-loop handling into dedicated `native_vello` modules while keeping public runtime entrypoints stable.

### 2. [x] Split `vendor/radiant/src/gui/native_shell/state/toolbar_helpers.rs` by toolbar family
- ROI / Effort: High / M
- Why it matters: Toolbar geometry and action assembly for browser, waveform, and top-bar controls are still concentrated in one oversized helper file, making UI changes noisy and increasing coupling between unrelated chrome surfaces.
- Evidence: `vendor/radiant/src/gui/native_shell/state/toolbar_helpers.rs` is about 1099 LOC. The file builds browser toolbar chips, waveform toolbar buttons, and top-bar/status helper geometry in one module.
- Recommended change: Split into focused browser-toolbar, waveform-toolbar, and top-bar helper modules with shared token/geometry helpers factored into a small common layer.
- Risk / tradeoffs: Moderate cache-key and hit-test wiring churn; keep button order, icon choice, and action payloads byte-for-byte stable.
- Suggested validation: Targeted `radiant` toolbar hit-test/render tests plus `ci_quick.ps1`.
- Completed: `2026-03-11` — `vendor/radiant` commit `c1d68d7a` split browser-toolbar, waveform-toolbar, waveform-visual, top-bar, browser-row, and sidebar helper families into focused `toolbar_helpers/*` modules while keeping the parent `state` call surface stable.

### 3. [x] Split `vendor/radiant/src/gui/native_shell/state/frame_build.rs` into smaller paint builders
- ROI / Effort: High / L
- Why it matters: Static-frame assembly remains oversized even after previous splits, which keeps browser/map/waveform/chrome rendering concerns coupled and makes cache invalidation work harder to reason about.
- Evidence: `vendor/radiant/src/gui/native_shell/state/frame_build.rs` is about 997 LOC, and its sub-builders such as `frame_build/browser.rs` and `frame_build/chrome.rs` are also still very large.
- Recommended change: Push more work into focused browser/map/waveform/status/chrome builders and keep `frame_build.rs` as a thin orchestration façade with small context structs only.
- Risk / tradeoffs: Paint-order regressions are easy to introduce; keep segment ordering and overlay/static composition invariant.
- Suggested validation: Targeted `radiant` frame/segment tests, screenshot/golden coverage where available, and `ci_quick.ps1`.
- Completed: `2026-03-11` — `vendor/radiant` commit `bcaaef6d` moved status-bar painting into `frame_build/status_bar.rs`, extracted state-overlay rendering into `frame_build/overlay.rs`, moved waveform BPM-grid painting into `frame_build/waveform.rs`, and reduced `frame_build.rs` to orchestration/context responsibilities.

### 4. [x] Finish slimming `vendor/radiant/src/gui/native_shell/state.rs` root state façade
- ROI / Effort: High / L
- Why it matters: The root native-shell state file still mixes mutable interaction state, hit-testing, frame build entrypoints, animation bookkeeping, and cache access, so changes to one area keep pulling broad rebuilds and test churn.
- Evidence: `vendor/radiant/src/gui/native_shell/state.rs` is about 1694 LOC and still owns `NativeShellState`, hit-test entrypoints such as `browser_row_at_point`, and frame-build entrypoints such as `build_frame_with_style`.
- Recommended change: Keep `state.rs` as a façade plus state struct definition only, and move the remaining hit-testing, overlay assembly, and browser/waveform interaction helpers into dedicated submodules.
- Risk / tradeoffs: Large internal API movement inside `radiant`; must preserve scene-cache fingerprints and hit-test semantics.
- Suggested validation: Existing native-shell state tests, targeted hit-test regressions, and `ci_quick.ps1`.
- Completed: `2026-03-11` — `vendor/radiant` commit `412426a9` moved remaining hit-testing, hover-resolution, motion-overlay, and playhead-trail logic into focused `state/*` modules so `state.rs` is reduced to the `NativeShellState` façade plus small lifecycle/build wrappers.

### 5. [x] Decompose embedding backfill orchestration in `src/app/controller/library/analysis_jobs/pool/job_execution/backfill.rs`
- ROI / Effort: High / M
- Why it matters: Embedding backfill still combines payload parsing, cache reuse, DB lookups, filesystem path resolution, worker fan-out, retry policy, and result persistence in one module, which raises bug risk in a high-value background pipeline.
- Evidence: `src/app/controller/library/analysis_jobs/pool/job_execution/backfill.rs` is about 525 LOC. `run_embedding_backfill_job`, `build_backfill_plan`, cache lookup helpers, and result persistence all live together.
- Recommended change: Split into planning, cache/repository access, worker execution, and persistence/reporting modules behind a thin job façade.
- Risk / tradeoffs: Moderate; DB/cache behavior must stay stable and duplicate-content reuse must not regress.
- Suggested validation: Existing backfill tests, worker retry tests, ANN/update integration tests, and `ci_quick.ps1`.
- Completed: `2026-03-11` — main repo commit `f8dbd240` replaced the single `backfill.rs` file with focused `backfill/{planning,repository,workers,persistence,model}.rs` modules while keeping `run_embedding_backfill_job` as a thin orchestration façade.

### 6. [x] Split analysis job claim/worker orchestration in `src/app/controller/library/analysis_jobs/pool/job_claim/mod.rs`
- ROI / Effort: High / M
- Why it matters: Decoder/compute worker setup, lease management, queue coordination, wakeups, and OS-specific thread tuning still meet in one module, obscuring failure paths and making worker-lifecycle changes risky.
- Evidence: `src/app/controller/library/analysis_jobs/pool/job_claim/mod.rs` is about 482 LOC. `spawn_decoder_worker` and `spawn_compute_worker` each carry broad orchestration responsibilities on top of nested module helpers.
- Recommended change: Split worker-thread loops, lifecycle/bootstrap, and OS-priority/wakeup glue into separate modules, leaving `mod.rs` as a small export surface.
- Risk / tradeoffs: Moderate concurrency risk; preserve shutdown, cancellation, and inflight-dedupe semantics.
- Suggested validation: Existing job-claim pool tests, decode heartbeat tests, and `ci_quick.ps1`.
- Completed: `2026-03-11` — main repo commit `072fb0ca` moved worker contexts into `job_claim/context.rs`, split decoder and compute worker orchestration into dedicated modules, isolated OS-priority glue in `job_claim/priority.rs`, and reduced `job_claim/mod.rs` to the module/export surface.

### 7. [x] Finish separating map view orchestration from repository and job flows
- ROI / Effort: High / M
- Why it matters: The map-view path still spreads user actions, DB connection policy, cluster/layout job submission, and legacy-named helper types across controller and repository modules, making future map work expensive.
- Evidence: `src/app/controller/ui/map_view.rs` is about 483 LOC and `src/app/controller/ui/map_view/repository.rs` is about 443 LOC. The controller still owns tab toggles, focus/preview behavior, cluster/layout enqueues, and DB-opening helpers, while repository code still contains repeated query-shape construction and `umap` compatibility naming.
- Recommended change: Split into controller actions, runtime jobs, source-DB access, and query/persistence modules, and continue narrowing legacy `umap` naming to explicit compatibility shims only.
- Risk / tradeoffs: Moderate; map focus/preview and source-scoped DB fallback behavior must remain stable.
- Suggested validation: Map-view tests, bounds/points fallback regressions, admin layout build tests, and `ci_quick.ps1`.
- Completed: `2026-03-11` — main repo commit `17d911b2` converted `ui/map_view.rs` into a module tree with separate controller, jobs, connection-policy, model, and repository query modules, and split the repository SQL helpers into focused bounds/points/clusters loaders while preserving the public `map_view` facade.

### 8. [ ] Split the waveform render pipeline around cached viewport, fade preview, and line paint backends
- ROI / Effort: High / M
- Why it matters: The public waveform renderer still contains a long decision tree for empty/full/peaks/cached/fade cases, which makes rendering behavior and performance-sensitive code paths difficult to reason about.
- Evidence: `src/waveform/render.rs` is about 542 LOC and `src/waveform/render/paint/lines.rs` is about 503 LOC. `render_color_image_for_view_with_size_and_fade` and `render_color_image_with_size` still branch across many concerns.
- Recommended change: Split viewport normalization, cache selection, fade-preview application, and paint backend dispatch into focused modules with a small public façade.
- Risk / tradeoffs: Moderate; waveform rendering is visual and performance-sensitive, so behavior drift is easy to miss without focused tests.
- Suggested validation: Existing waveform render/cache tests, fade-tail regressions, any benchmark smoke tests, and `ci_quick.ps1`.

### 9. [ ] Finish splitting `src/app/controller/playback/waveform_actions/mod.rs` into smaller adapter surfaces
- ROI / Effort: Medium-High / M
- Why it matters: The earlier waveform-action split helped, but the façade still carries a large number of UI milli/micro adapters plus selection/edit-selection entrypoints in one file.
- Evidence: `src/app/controller/playback/waveform_actions/mod.rs` is about 443 LOC and still contains many `set_waveform_*` adapter methods for selection, edit selection, seek, cursor, and smart-scale behaviors.
- Recommended change: Split the façade into selection adapters, edit-selection adapters, seek/cursor adapters, and shared unit-conversion helpers while keeping the `AppController` call surface stable.
- Risk / tradeoffs: Low-to-moderate; mostly structural, but waveform input routing is user-visible and heavily exercised.
- Suggested validation: Existing waveform controller tests, selection BPM/smart-scale regressions, and `ci_quick.ps1`.

### 10. [ ] Separate audio recording capture, writer, and live-monitor workers in `src/audio/recording.rs`
- ROI / Effort: Medium / M
- Why it matters: Recording still bundles CPAL stream startup, WAV writer worker management, live monitor relay, and thread teardown behavior in one file, which complicates failure-path testing and future device work.
- Evidence: `src/audio/recording.rs` is about 351 LOC. `AudioRecorder`, `RecorderWriter`, `InputMonitor`, command enums, and the worker loops all live together.
- Recommended change: Split capture/bootstrap, WAV writing, monitor plumbing, and shared command/state definitions into focused modules with clearer ownership boundaries.
- Risk / tradeoffs: Moderate; thread teardown and monitor replay need to remain non-blocking and panic-safe.
- Suggested validation: Existing recording tests, monitor attach/detach coverage, and `ci_quick.ps1`.

### 11. [ ] Split ANN container format handling in `src/analysis/ann_index/container.rs`
- ROI / Effort: Medium / M
- Why it matters: The ANN container implementation mixes header encoding/validation, streaming copy helpers, tempfile persistence, and unpack logic in one file, which makes future format evolution and corruption handling harder.
- Evidence: `src/analysis/ann_index/container.rs` is about 465 LOC. `AnnContainerHeader`, `write_container`, `unpack_container`, header parsing, and copy/read helpers all live together.
- Recommended change: Split into header/codec, IO streaming, and container façade modules with clearer boundary tests.
- Risk / tradeoffs: Moderate; any mistake can break index migration or corruption detection.
- Suggested validation: Existing ANN container tests, ANN migration/load tests, and `ci_quick.ps1`.

### 12. [ ] Separate similarity-map layout/reporting façade from the t-SNE engine in `src/analysis/umap.rs`
- ROI / Effort: Medium / M
- Why it matters: The public layout module still uses a legacy `umap` compatibility shell around a t-SNE implementation, and it mixes report types, SQL load/write behavior, projection, and validation in one place.
- Evidence: `src/analysis/umap.rs` is about 358 LOC. `build_map_layout`, `build_umap_layout`, report-path helpers, embedding loading, `compute_tsne`, and validation all live together.
- Recommended change: Keep the public compatibility façade small and move projection math, DB load/write helpers, and report serialization into dedicated modules with explicit docs about the compatibility contract.
- Risk / tradeoffs: Low-to-moderate; must preserve persisted schema names and CLI compatibility.
- Suggested validation: Existing similarity-map tests, admin CLI tests, and `ci_quick.ps1`.

### 13. [ ] Break `src/app_core/native_bridge/tests/projection_cache.rs` into segment-focused test modules
- ROI / Effort: Medium-High / M
- Why it matters: Projection-cache testing is still concentrated in one very large file full of repeated key-drift cases, which makes coverage hard to navigate and encourages more accumulation in the same hub.
- Evidence: `src/app_core/native_bridge/tests/projection_cache.rs` is about 608 LOC and contains cache-key drift, segment invalidation, and reuse tests for many unrelated surfaces in one module.
- Recommended change: Split into key-drift, segment reuse, waveform projection, browser projection, and options/progress cache modules with shared controller fixtures.
- Risk / tradeoffs: Low; mostly structural test work, but keep fixture setup consistent so failures remain easy to interpret.
- Suggested validation: `cargo test app_core::native_bridge::tests -- --test-threads=1` and `ci_quick.ps1`.

### 14. [ ] Break remaining controller test hubs into behavior modules
- ROI / Effort: Medium / M
- Why it matters: Some of the earlier test-hub cleanup landed, but controller behavior is still concentrated in a few oversized mixed-topic files.
- Evidence: `src/app_core/controller/tests.rs` is about 459 LOC and `src/app/controller/tests/browser_core.rs` is about 410 LOC. Both files still mix unrelated behavior families in one place.
- Recommended change: Split by behavior domain (native action routing, projection/state transitions, browser filters/focus/loading, etc.) with shared fixture helpers extracted once.
- Risk / tradeoffs: Low; mostly structural, but preserve test naming and discovery.
- Suggested validation: Targeted controller test modules plus `ci_quick.ps1`.

### 15. [ ] Split controller audio option normalization from UI projection in `src/app/controller/playback/audio_options.rs`
- ROI / Effort: Medium / S-M
- Why it matters: Input/output device normalization logic is reusable and testable on its own, but the file still mixes pure normalization, probing policy, settings mutation, and UI projection in one place.
- Evidence: `src/app/controller/playback/audio_options.rs` is about 445 LOC. `normalize_audio_options`, output refresh, input refresh, and channel-probing logic are still bundled together.
- Recommended change: Move pure normalization/probing policy into focused helpers and keep the controller methods limited to settings/UI synchronization.
- Risk / tradeoffs: Low; behavior should stay identical if pure helpers are extracted carefully.
- Suggested validation: Focused normalization tests, audio option refresh tests, and `ci_quick.ps1`.

### 16. [ ] Refresh stale cleanup metadata and file-size debt allowlists
- ROI / Effort: High / S
- Why it matters: The current handoff and debt-tracking docs still describe the previous cleanup pass as current, and the file-size allowlist still references many files or split surfaces that no longer exist in that form.
- Evidence: `src/lib.rs` still says the map-view split is pending in old cleanup item 19; `vendor/radiant/src/lib.rs` still references completed cleanup items 20-23/26/30 as pending; `docs/file_size_budget_allowlist.txt` still lists old paths such as `src/app_core/native_bridge.rs`, `src/audio/output.rs`, `src/waveform/mod.rs`, and `vendor/radiant/src/app/mod.rs`; `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md` still frame the old cleanup lane as the current one.
- Recommended change: Replace stale cleanup references with the new backlog, prune obsolete allowlist entries, and regenerate/refresh the cleanup hotspot snapshot as part of the next maintenance pass.
- Risk / tradeoffs: Low; documentation-only, but drift here directly hurts wake-up clarity and future planning accuracy.
- Suggested validation: `scripts/check_docs_index.ps1`, `scripts/check_markdown_links.ps1` if docs links move, and `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`.
