## Goal
- Plan a per-source single-file ANN index format that sits alongside each source `library.db`, matching current HNSW performance and flexibility.

## Proposed solutions
- Implement a custom single-file container that stores HNSW graph/data plus id map with fixed offsets for mmap-friendly access.
- Keep HNSW serialization but add a thin wrapper that packs/unpacks the two HNSW files and id map into one file without changing the search algorithm.
- Add versioned header metadata to support future format changes and multiple embedding models.

## Step-by-step plan
1. [x] Audit the current ANN storage flow (file paths, HNSW dump/load, id map handling) and confirm where the per-source DB path is resolved.
2. [x] Define the single-file container format (header, version, model_id, offsets, lengths, checksum) and decide binary vs JSON for the id map.
3. [x] Add a new storage module to read/write the container file, keeping mmap-friendly layout and minimal copying.
4. [x] Update ANN build/load paths to use the container file next to `library.db`, with fallback to legacy files and automatic migration.
5. [x] Add tests for container round-trip, migration from old files, and consistency with existing ANN search results.
6. [-] Benchmark load/search performance vs the current multi-file approach and confirm parity.
7. [x] Document the new file format, migration behavior, and any cleanup tooling.

## Code Style & Architecture Rules Reminder
- Keep files under 400 lines; split when necessary.
- When functions require more than 5 arguments, group related values into a struct.
- Each module must have one clear responsibility; split when responsibilities mix.
- Do not use generic buckets like `misc.rs` or `util.rs`. Name modules by domain or purpose.
- Name folders by feature first, not layer first.
- Keep functions under 30 lines; extract helpers as needed.
- Each function must have a single clear responsibility.
- Prefer many small structs over large ones.
- All public objects, functions, structs, traits, and modules must be documented.
- All code should be well tested whenever feasible.
- “Feasible” should be interpreted broadly: tests are expected in almost all cases.
- Prefer small, focused unit tests that validate behaviour clearly.
- Do not allow untested logic unless explicitly approved by the user.

## Layout Redesign (Phase 6) Checklist

Goal: migrate high-impact native-shell geometry to strict slotized helper
adapters while preserving current visual and interaction behavior.

1. [x] Slotize top-bar bands (`title row`, `controls row`, `title cluster`,
   `action cluster`) in `layout_adapter`.
2. [x] Slotize browser bands (`tabs`, `toolbar`, `header`, `rows`, `footer`)
   in `layout_adapter`.
3. [x] Slotize sidebar bands (`header`, `rows`, `footer`) in `layout_adapter`.
4. [x] Route sidebar source/folder split via `layout_adapter` section helper
   instead of shell-state local split helpers.
5. [x] Rewire `ShellLayout::build_with_style(...)` to consume slotized band
   outputs for top-bar/browser/sidebar.
6. [x] Update layout spec status + tracked remaining gap text in
   `docs/radiant_slot_layout_spec.md`.

## Layout Redesign (Phase 7) Checklist

Goal: migrate remaining overlay and control-strip micro-layout surfaces to
strict slotized helper adapters while preserving current behavior.

1. [x] Add slotized overlay adapters for prompt/progress/drag geometry and
   route state hit-testing/rendering through adapter outputs.
2. [x] Add slotized control-strip adapters for top-bar update actions, browser
   actions, and sidebar footer actions.
3. [x] Add slotized browser toolbar section adapter for search/activity/sort
   partitioning left of the action strip.
4. [x] Split new adapter code into focused modules under the line-budget
   constraints and add focused unit tests for non-trivial geometry logic.
5. [x] Regenerate native-shell shot fixtures and keep `vendor/radiant` tests
   green after rewiring.
6. [x] Update layout redesign docs/spec status for this phase.

## Layout Redesign (Phase 8) Checklist

Goal: migrate sidebar header text/badge/divider micro-layout into strict
slotized adapters so shell-state rendering no longer owns local rect math for
that surface.

1. [x] Add a focused `layout_adapter::sidebar_header` module for folder-header
   text rows, recovery badge geometry/label compaction, and source-section
   divider placement.
2. [x] Rewire shell-state sidebar rendering/tests to consume adapter-owned
   folder-header and divider outputs.
3. [x] Remove superseded sidebar header/divider helper arithmetic from
   `state.rs` and keep behavior deterministic through adapter contracts.
4. [x] Regenerate affected native-shell shot fixtures and keep
   `vendor/radiant` tests green.
5. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 9) Checklist

Goal: migrate status-bar segment/text-line geometry into strict slotized
adapters so shell layout/state no longer own local status rect arithmetic.

1. [x] Add a focused `layout_adapter::status_bar` module for status
   left/center/right segment geometry.
2. [x] Route `ShellLayout::build_with_style(...)` status segment computation
   through adapter outputs instead of local proportional math.
3. [x] Add slotized status text-line rect helper and route status text rendering
   + motion overlay rendering through adapter-computed bounds.
4. [x] Add focused adapter tests for status segment ordering/clamping and status
   text-line bounds constraints.
5. [x] Regenerate affected native-shell shot fixtures and keep
   `vendor/radiant` tests green.
6. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 10) Checklist

Goal: migrate waveform header title/metadata text-row geometry into strict
slotized adapters so shell-state rendering no longer owns local waveform header
rect arithmetic.

1. [x] Add a focused `layout_adapter::waveform_header` module for waveform
   header title and metadata row geometry.
2. [x] Route native-shell waveform header text rendering through
   adapter-computed text-row rects.
3. [x] Add focused adapter tests for waveform header row ordering/bounds and
   empty-header collapse behavior.
4. [x] Regenerate affected native-shell shot fixtures and keep
   `vendor/radiant` tests green.
5. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 11) Checklist

Goal: migrate browser list table-header and row text/chip geometry into strict
slotized adapters so shell-state rendering no longer owns local browser
label/baseline rect arithmetic for those surfaces.

1. [x] Add a focused `layout_adapter::browser_text` module for browser table
   columns, table-header label bounds, and row text/chip bounds.
2. [x] Route browser list rendering and focused-row overlay text rendering
   through adapter-computed browser text/chip rects.
3. [x] Add focused adapter tests for browser column ordering and
   row/header text bounds constraints.
4. [x] Regenerate affected native-shell shot fixtures and keep
   `vendor/radiant` tests green.
5. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 12) Checklist

Goal: migrate map-active browser-header metadata text geometry into strict
slotized adapters so shell-state rendering no longer owns local map-header
text baseline and right-anchor rect arithmetic.

1. [x] Add a focused `layout_adapter::map_header` module for map-header
   left/right metadata label bounds.
2. [x] Route map-active browser-header rendering through adapter-computed
   map-header label rects.
3. [x] Add focused adapter tests for map-header bounds and right-partition
   constraints.
4. [x] Keep `vendor/radiant` tests green after the rewiring.
5. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 13) Checklist

Goal: migrate top-bar update status/controls text geometry into strict
slotized adapters so shell-state rendering no longer owns local action-cluster
reserved-width and baseline rect arithmetic for that surface.

1. [x] Add a focused `layout_adapter::update_text` module for top-bar update
   status and controls text-line bounds.
2. [x] Route top-bar update text rendering through adapter-computed line rects
   instead of local reserved-width and text-top calculations.
3. [x] Add focused adapter tests for update text bounds, button-reservation
   behavior, and empty-cluster collapse.
4. [x] Keep `vendor/radiant` tests green after the rewiring.
5. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 14) Checklist

Goal: migrate prompt/progress/drag overlay copy geometry into strict slotized
adapters so shell-state overlay rendering no longer owns local title/detail/
input/error/button-label baseline arithmetic for those surfaces.

1. [x] Add a focused `layout_adapter::overlays::text` module for prompt,
   progress, and drag overlay text-line bounds.
2. [x] Route prompt/progress/drag text rendering through adapter-computed
   text-line rects instead of local y-offset and text-top calculations.
3. [x] Add focused adapter tests for overlay text bounds and optional-row
   behaviors (detail/target/input-error).
4. [x] Regenerate affected native-shell shot fixtures and keep
   `vendor/radiant` tests green.
5. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 15) Checklist

Goal: migrate browser chrome tabs/toolbar/footer copy geometry into strict
slotized adapters so shell-state browser rendering no longer owns local
label-baseline arithmetic for those surfaces.

1. [x] Add a focused `layout_adapter::browser_chrome_text` module for browser
   tabs labels, toolbar chip/field labels, and footer summary label bounds.
2. [x] Route browser chrome text rendering through adapter-computed text-line
   rects in both full-frame and state-overlay render paths.
3. [x] Add focused adapter tests for browser chrome text bounds and
   empty-section collapse behavior.
4. [x] Keep `vendor/radiant` tests green after the rewiring.
5. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 16) Checklist

Goal: migrate control-strip label text geometry into strict slotized adapters
so shell-state no longer owns local button-label and top-controls baseline
arithmetic for those surfaces.

1. [x] Add a focused `layout_adapter::control_text` module for top-bar controls
   labels and action-button label bounds.
2. [x] Route update/browser/sidebar action-button text rendering through
   adapter-computed label rects.
3. [x] Route top-bar controls labels (`Options`, volume value, `Vol`) through
   adapter-computed text-line rects.
4. [x] Add focused adapter tests for control text bounds and empty-button
   collapse behavior.
5. [x] Regenerate affected native-shell shot fixtures and keep
   `vendor/radiant` tests green.
6. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 17) Checklist

Goal: migrate sidebar source/folder row and recovery-badge copy geometry into
strict slotized adapters so shell-state no longer owns local row-label/badge
baseline arithmetic for those paths.

1. [x] Add a focused `layout_adapter::sidebar_text` module for source-row,
   folder-row, and recovery-badge label bounds.
2. [x] Route sidebar source/folder row text rendering through adapter-computed
   label rects in full-frame rendering.
3. [x] Route focused folder-row overlay text rendering through adapter-computed
   label rects in state-overlay rendering.
4. [x] Route recovery-badge label rendering through adapter-computed label
   rects.
5. [x] Add focused adapter tests for sidebar row/badge text bounds and indent
   behavior.
6. [x] Regenerate affected native-shell shot fixtures and keep
   `vendor/radiant` tests green.
7. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 18) Checklist

Goal: migrate top-bar title copy geometry into strict slotized adapters so
shell-state no longer owns local top-title baseline and inset arithmetic.

1. [x] Add a focused `layout_adapter::top_title_text` module for top-bar title
   label bounds.
2. [x] Route top-bar title rendering through adapter-computed label rects in
   full-frame rendering.
3. [x] Remove legacy `text_top_in_rect(...)` helper usage from native-shell
   state rendering.
4. [x] Add focused adapter tests for top-bar title bounds and inset behavior.
5. [x] Regenerate affected native-shell shot fixtures and keep
   `vendor/radiant` tests green.
6. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 19) Checklist

Goal: migrate sidebar header/footer chrome copy geometry into strict slotized
adapters so shell-state no longer owns local sidebar chrome text-row arithmetic.

1. [x] Add a focused `layout_adapter::sidebar_chrome_text` module for sidebar
   header title/query and footer primary/secondary line bounds.
2. [x] Route sidebar header title/query rendering through adapter-computed line
   rects in full-frame rendering.
3. [x] Route sidebar footer summary/recovery rendering through adapter-computed
   line rects in full-frame rendering.
4. [x] Add focused adapter tests for sidebar chrome text bounds and ordering.
5. [x] Regenerate affected native-shell shot fixtures and keep
   `vendor/radiant` tests green.
6. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 20) Checklist

Goal: migrate browser-row label truncation width resolution to adapter-owned
slot geometry so browser virtualization no longer relies on local width
arithmetic helpers.

1. [x] Route `rendered_browser_rows(...)` sample-label truncation width through
   `compute_browser_row_text_layout(...)` sample-label bounds.
2. [x] Remove legacy local `row_label_width(...)` helper usage from browser-row
   cache construction.
3. [x] Add a focused regression test asserting browser-row truncation uses the
   slotized sample-label width.
4. [x] Regenerate affected native-shell shot fixtures (if changed) and keep
   `vendor/radiant` tests green.
5. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 21) Checklist

Goal: migrate browser-map canvas geometry into adapter-owned helpers so
native-shell map rendering and hit-testing no longer rely on local canvas/point
placement arithmetic in `state.rs`.

1. [x] Add a focused `layout_adapter::map_canvas` module for browser-map canvas
   bounds and normalized map-point center resolution.
2. [x] Route map-active canvas rendering through adapter-computed canvas/point
   geometry.
3. [x] Route map hit-testing (`map_sample_id_at_point`) through adapter-computed
   canvas/point geometry.
4. [x] Remove legacy local map canvas/point helper arithmetic from `state.rs`.
5. [x] Add focused adapter tests for map-canvas bounds and point clamping.
6. [x] Regenerate affected native-shell shot fixtures (if changed) and keep
   `vendor/radiant` tests green.
7. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 22) Checklist

Goal: migrate browser tab surface geometry into adapter-owned helpers so
native-shell tab rendering and tab hit-testing no longer rely on local
split-width arithmetic in `state.rs`.

1. [x] Add a focused `layout_adapter::browser_tabs` module for tab rect
   geometry (`samples`/`map`) with slotized row distribution.
2. [x] Route browser-tab rendering through adapter-computed tab rects.
3. [x] Route browser-tab hit-testing (`browser_tab_action_at_point`) through
   adapter-computed tab rects.
4. [x] Remove legacy local browser-tab split helper arithmetic from `state.rs`.
5. [x] Add focused adapter tests for tab rect bounds and ordering.
6. [x] Regenerate affected native-shell shot fixtures (if changed) and keep
   `vendor/radiant` tests green.
7. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 23) Checklist

Goal: migrate row hit-testing from stride arithmetic to geometry-driven rect
resolution so pointer routing uses the actual cached slotized row bounds.

1. [x] Add a focused `layout_adapter::row_hit_test` module with rect-based row
   index resolution helper(s).
2. [x] Route source/folder row hit-testing through adapter row-index helpers.
3. [x] Route browser-row hit-testing through geometry-driven row containment
   checks instead of row-height/gap stride arithmetic.
4. [x] Remove legacy stride-based row-index helper arithmetic from `state.rs`.
5. [x] Add focused adapter tests for row-hit resolution and gap/empty behavior.
6. [x] Regenerate affected native-shell shot fixtures (if changed) and keep
   `vendor/radiant` tests green.
7. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 24) Checklist

Goal: migrate prompt/progress/drag overlay visual geometry into adapter-owned
helpers so native-shell rendering and hit-testing no longer depend on local
overlay rect arithmetic.

1. [x] Add a focused `layout_adapter::overlay_visuals` module for prompt,
   progress, and drag overlay visual rect outputs.
2. [x] Route overlay hit-testing helper paths (`prompt`/`progress`/`drag`)
   through adapter-owned visual geometry.
3. [x] Route progress overlay filled-track geometry through adapter-owned rect
   outputs instead of render-path local width math.
4. [x] Add focused adapter tests for overlay visual bounds/scrim/fill behavior.
5. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 25) Checklist

Goal: migrate waveform annotation visual geometry into adapter-owned helpers so
native-shell rendering no longer owns local selection/cursor/playhead rect
arithmetic.

1. [x] Add a focused `layout_adapter::waveform_annotations` module for
   selection/cursor/playhead rect computation from normalized milli anchors.
2. [x] Route waveform annotation rendering through adapter-computed rects.
3. [x] Add focused adapter tests for annotation bounds and edge clamping.
4. [x] Update slot-layout spec status and tracked remaining gap notes.

## Layout Redesign (Phase 26) Checklist

Goal: close the current tail milestone with deterministic coverage and updated
design status reporting.

1. [x] Keep new adapter modules under focused responsibilities and include
   public API docs for visual/annotation geometry contracts.
2. [x] Add/retain deterministic unit coverage for new non-trivial geometry
   paths.
3. [x] Update active plan + slot-layout spec to reflect the completed
   milestone and narrowed residual gap scope.

## Runtime Performance Redesign (Multi-Day) Checklist

Goal: improve native runtime responsiveness with deterministic invalidation and
cache-friendly projection/layout behavior, following Xilem-style scoped updates.

1. [x] Decouple overlay-only redraw rebuilds from unconditional full-model
   pulls in `native_vello` while preserving explicit startup model refresh.
2. [ ] Add explicit runtime invalidation scopes (full/static/state/motion) and
   route `UiAction` handling through scope classifiers instead of blanket
   full-scene dirties.
3. [ ] Introduce model/projection cache keys in `app_core` so unchanged
   browser/sidebar/map surfaces reuse prior projected view models.
4. [ ] Persist and reuse layout engine state in layout-core runtime integration
   so dirty-subtree invalidation APIs are exercised on hot paths.
5. [ ] Add focused performance telemetry gates and benchmarks for hover, wheel,
   map pan, and waveform interaction latencies.
