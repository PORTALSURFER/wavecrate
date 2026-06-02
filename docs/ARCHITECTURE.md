# Architecture and Ownership

This document is the durable architecture map for Wavecrate. It explains what the
system is optimizing for, where code should live, and how the Wavecrate and
Radiant boundaries are meant to stay clean over time.

## Product principles

Wavecrate is a realtime-oriented sample manager for exploratory listening,
selection, and curation of audio material. The architecture should preserve a
few non-negotiable product properties:

- realtime primacy
  - UI response should remain perceptually immediate
  - blocking work belongs off the UI thread
- flow preservation
  - reversible actions beat confirmation-heavy workflows
  - failures may surface, but the UI should not freeze
- audition-first design
  - playback, scrubbing, and navigation should stay fast enough to support
    listening as the primary interaction mode
- predictability over cleverness
  - state transitions should be observable and easy to reason about
- data trust
  - destructive flows need explicit recovery behavior
- performance as correctness
  - perceived slowness counts as a product defect, not a cosmetic issue

Use these principles when a change is ambiguous: prefer the calmer, more
predictable, and lower-latency design.

## Change routing rules

- domain workflows, persistence orchestration, and application state belong in
  `src/`
- default product-specific GUI behavior should live in `src/gui_app.rs` and
  `src/app_core/**`; do not reintroduce the removed legacy GUI path
- current default-GUI folder/file drag/drop behavior lives under
  `src/gui_app/folder_browser/**`; do not route product drag/drop fixes to
  `src/app/controller/ui/drag_drop_controller/**` unless the task explicitly
  names the deprecated compatibility controller
- reusable UI/runtime/layout work belongs in `vendor/radiant`
- runtime compatibility behavior should stay inside the runtime/test surfaces
  rather than leaking back into generic Radiant modules
- large import lists are architecture signals, not formatting problems. When a
  Wavecrate GUI module needs broad imports from unrelated UI, domain, runtime,
  and helper areas, first split the module by responsibility or move reusable
  GUI behavior into Radiant. Keep imports explicit, avoid wildcard imports
  outside tests/preludes, and avoid using facade modules as dumping grounds for
  state, view construction, side effects, and re-exports at the same time. A
  facade may wire focused modules together, but it should not become the owner
  of app state shape, widget construction, side effects, and reusable GUI
  helpers.

## Ownership map

### `src/app/controller/**`

Owns:

- UI intent handling
- controller orchestration
- library workflows and recovery actions

Should avoid:

- renderer-specific geometry logic
- low-level DB primitives

### `src/app_core/**`

Owns:

- host-facing application state projection
- UI bridge projection/invalidation rules
- GUI action catalog and runtime test integration

Should avoid:

- direct filesystem mutation policy outside the persistence layer
- new coupling back into the removed legacy UI boundary

### `src/gui_app.rs`

Owns:

- the default Wavecrate desktop GUI entrypoint
- composition of Radiant's current application, runtime, widget, and GPU-surface
  APIs for Wavecrate's sample-workstation UI
- current folder-browser folder/file drag/drop interactions, with support
  modules in `src/gui_app/folder_browser/**`

Should avoid:

- owning reusable Radiant behavior that should live in `vendor/radiant`
- reintroducing dependencies on the deprecated legacy GUI path

### `src/app/controller/ui/drag_drop_controller/**`

Owns:

- controller-level drag/drop behavior still exercised by compatibility tests

Should avoid:

- being used as the default target for current `src/gui_app.rs` product
  drag/drop bugs

### `src/sample_sources/**`

Owns:

- database schema and read/write APIs
- journal-backed file operations
- crash recovery behavior for file and folder mutations

Should avoid:

- UI policy and rendering behavior

### `src/issue_gateway/**`

Owns:

- issue reporting DTOs
- token storage and integration boundaries

### `vendor/radiant/**`

Owns:

- reusable layout primitives
- reusable widgets
- runtime/backend integration
- reusable runtime/test primitives used by Wavecrate's GUI

Should avoid:

- taking ownership of Wavecrate-specific controller or domain policy

## Radiant boundary

Radiant is moving toward a reusable GUI library with three preferred generic
areas:

- `radiant::layout`
- `radiant::widgets`
- `radiant::runtime`

Wavecrate's product UI is `src/gui_app.rs`. Support modules used by
compatibility and tests are not fallback product UIs and should not define new
behavior.

Practical rule:

- new generic GUI abstractions belong in the public Radiant layers
- compatibility fixes may still touch runtime/test infrastructure
- new Wavecrate product behavior should compose generic Radiant surfaces where
  possible instead of expanding Wavecrate-only runtime APIs

## UI design direction

The current UI direction is dense, structural, and utilitarian rather than
ornamental. Preserve these characteristics:

- strong layout structure over decorative flourish
- clear visual hierarchy and repeatable geometry
- interaction feedback that is obvious but not noisy
- stable surfaces that prioritize readability and manipulation over novelty

If a style decision would trade clarity or responsiveness for decoration, do
not take that trade.

## CODEOWNERS and guardrails

When ownership boundaries change, update both:

- `.github/CODEOWNERS`
- the relevant architecture notes in this file

Guardrails run through `scripts/check.{sh,ps1}` subcommands backed by
`scripts/internal/check/`. When one fires, fix the ownership violation before
considering an allowlist.
