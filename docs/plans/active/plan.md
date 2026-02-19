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
