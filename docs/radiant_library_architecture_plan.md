# Radiant Library Architecture Plan

Status: draft architecture plan for `OPT-30`

Purpose: define the target public architecture for `vendor/radiant` as a
reusable declarative GUI library, while keeping Sempal shippable throughout the
transition.

This document is intentionally about boundaries and migration shape. It does
not authorize behavior changes by itself.

## Why this plan exists

The current `Sempal` / `Radiant` split is only partially generic:

- `Radiant` already contains a strong generic slot-based layout engine.
- `Radiant` also still exposes a public API centered on Sempal-specific shell
  models such as `AppModel`, `UiAction`, and `native_shell`.
- `Sempal` consumes that shell-shaped API cleanly, but that does not yet make
  `Radiant` a reusable GUI library for other applications.

The goal of this plan is to make the intended end state explicit before
implementation work starts.

## Design goals

- Make `Radiant` reusable outside Sempal.
- Keep one-way declarative data flow.
- Draw a hard architectural line between:
  - layout containers that own child placement
  - widgets that represent interactive end-user controls
- Keep `Sempal` shippable during migration by treating the current shell as a
  compatibility layer instead of removing it early.
- Avoid a big-bang rewrite.

## Non-goals

- Rewriting the entire Sempal UI in one pass.
- Removing the current native shell immediately.
- Designing a fully general retained scene graph beyond what the current
  product and library need.
- Changing product behavior as part of this planning document.

## Target architecture

The target `Radiant` package structure should be:

1. `radiant::layout`
   - Generic layout engine and container primitives.
   - Owns constraints, slots, measurement, layout, overflow, and
     virtualization.
   - Knows nothing about Sempal screens, browser rows, waveform panels, or
     update prompts.

2. `radiant::widgets`
   - Generic interactive and presentational controls.
   - Owns widget props, intrinsic sizing, interaction state contracts, focus
     participation, and message emission.
   - Built on top of `layout`, not inside app-specific shell code.

3. `radiant::runtime`
   - Host/runtime integration layer.
   - Owns event normalization, window lifecycle, scheduling, repaint signals,
     and backend rendering orchestration.
   - Consumes generic declarative view trees and host-defined message types.

4. `radiant::compat::sempal_shell` or equivalent compatibility namespace
   - Transitional home for the current Sempal-shaped shell contract and native
     shell implementation.
   - Explicitly not the preferred public API for new host applications.
   - Exists to keep Sempal running while the generic surfaces mature.

## Containers vs widgets

This distinction is the core architectural rule.

### Containers

Containers own child layout. They decide:

- child slot ownership
- main-axis and cross-axis distribution
- spacing, padding, margins, and overflow behavior
- clipping and virtualization policy where applicable

Examples:

- `PaddingBox`
- `AlignBox`
- `Row`
- `Column`
- `Grid`
- `Stack`
- `ScrollView`
- `Wrap`

Containers may contain children. They do not represent end-user intent by
themselves.

### Widgets

Widgets represent presentational or interactive end-user elements. They decide:

- intrinsic size
- visual states
- focusability
- pointer and keyboard interaction semantics
- emitted messages or callbacks

Examples:

- `Text`
- `Button`
- `Toggle`
- `TextInput`
- `Scrollbar`
- `ListRow`
- `Canvas`

Widgets should be leaf nodes or tightly bounded composites. They should not own
general child layout policy beyond clearly scoped internal composition.

### Composition rule

The public programming model should be:

- apps compose containers and widgets declaratively
- runtime normalizes input and routes it deterministically
- widgets emit host-defined messages
- the host reduces those messages into state
- the next view tree is projected from host state

The current closure-driven declarative bridge is directionally correct. The
main missing piece is that the public contract is still dominated by a
Sempal-shaped app snapshot rather than a generic view/message surface.

## Ownership boundary

### Radiant core should own

- generic layout primitives
- generic widgets
- input normalization
- focus, hit testing, pointer capture, and key routing
- repaint and scene scheduling
- backend rendering/runtime integration
- generic automation/debug surfaces that operate on generic widget roles and
  node identities

### Sempal should own

- domain state
- domain actions and reducers
- sample-library workflows
- browser/search/similarity semantics
- waveform editing semantics
- updater/install flows
- Sempal-specific screen composition and product copy

### Compatibility layer should own

- current Sempal native shell model types
- shell geometry and rendering built around Sempal regions
- migration adapters from old shell projections into newer generic Radiant
  surfaces

## Surface inventory

The table below classifies the main current `Radiant` surfaces.

| Current surface | Current role | Classification | Target home |
| --- | --- | --- | --- |
| `vendor/radiant/src/gui/layout_core/*` | Generic slot-based layout engine | Generic, keep in core | `radiant::layout` |
| `vendor/radiant/src/gui/input.rs` | Input tokens and normalization vocabulary | Generic, keep in core | `radiant::runtime` with shared public input types |
| `vendor/radiant/src/gui/repaint.rs` | Repaint signaling | Generic, keep in core | `radiant::runtime` |
| `vendor/radiant/src/gui/types.rs` | Geometry and image/color primitives | Generic, keep in core | shared public core/types module |
| `vendor/radiant/src/gui/native_shell/layout_adapter/*` | Shell geometry adapters using layout core | Sempal-specific | compatibility layer |
| `vendor/radiant/src/gui/native_shell/layout/*` | Retained shell tree with Sempal regions | Sempal-specific | compatibility layer |
| `vendor/radiant/src/gui/native_shell/state/*` | Shell interaction/render state for browser/sidebar/waveform/update flows | Sempal-specific | compatibility layer |
| `vendor/radiant/src/gui/native_shell/style/*` | Current shell-specific tokens and chrome sizing | Generic but needs redesign | split into generic theming/tokens plus compatibility overrides |
| `vendor/radiant/src/app/declarative.rs` | Closure-driven declarative bridge | Generic, keep with redesign | `radiant::runtime` or generic host bridge module |
| `vendor/radiant/src/app/bridge.rs` | Host/runtime bridge trait | Generic but needs redesign | generic runtime bridge |
| `vendor/radiant/src/app/dirty_segments.rs` | Incremental rebuild hints | Generic but needs redesign | generic runtime/render invalidation surface |
| `vendor/radiant/src/app/automation.rs` | Automation snapshot roles and nodes | Generic but needs redesign | generic runtime/automation surface |
| `vendor/radiant/src/app/actions/mod.rs` | Giant Sempal-specific `UiAction` enum | Sempal-specific | compatibility layer until replaced |
| `vendor/radiant/src/app/browser.rs` | Browser/list/map models | Sempal-specific | Sempal or compatibility layer |
| `vendor/radiant/src/app/shell.rs` | Top-level shell app model | Sempal-specific | compatibility layer |
| `vendor/radiant/src/app/sources.rs` | Source/folder pane models | Sempal-specific | Sempal or compatibility layer |
| `vendor/radiant/src/app/waveform.rs` | Waveform-facing shell models | Sempal-specific | Sempal or compatibility layer |
| `vendor/radiant/src/app/waveform_tempo.rs` | Tempo parsing helper shaped around Sempal waveform flows | Sempal-specific | Sempal domain layer |
| `vendor/radiant/src/gui_runtime/mod.rs` | Window/runtime API | Generic, keep in core | `radiant::runtime` |
| `vendor/radiant/src/gui_runtime/native_vello.rs` | Current native runtime loop | Generic but needs layering cleanup | `radiant::runtime::native_vello` |

## Resulting public API shape

The target public API should read more like:

```rust
use radiant::{
    layout::{Column, Row, ScrollView},
    runtime::{AppRuntime, View, WidgetId},
    widgets::{Button, Text, TextInput, Toggle},
};
```

and less like:

```rust
use radiant::app::{AppModel, BrowserPanelModel, UiAction, WaveformPanelModel};
```

That is the core architectural shift.

## Migration strategy

The migration should happen in phases.

### Phase 0: architecture contract

Goal:

- write down the target library shape and migration path

Outputs:

- this architecture plan
- issue breakdown for implementation lanes

Blast radius:

- docs only

### Phase 1: publish the generic layout core

Goal:

- make the slot-based layout engine a supported public API

Includes:

- promote `layout_core`
- tighten naming and docs
- keep shell-only adapters out of the exported layout surface

Dependency:

- none beyond Phase 0

Mapped issue:

- `OPT-31`

### Phase 2: define the widget model

Goal:

- define what a widget is in Radiant and how it differs from a container

Includes:

- widget taxonomy
- shared widget responsibilities
- declarative composition expectations

Dependencies:

- Phase 0
- Phase 1

Mapped issue:

- `OPT-32`

Status:

- Implemented as the first public `radiant::widgets` contract surface.
- Code-facing taxonomy and contracts now live in
  `vendor/radiant/src/widgets/*`.
- Composition guidance and examples now live in
  `docs/radiant_widget_model.md`.

### Phase 3: implement the first reusable primitive widgets

Goal:

- create the first usable generic widget set

Includes:

- button
- toggle
- text input
- scrollbar

Dependencies:

- Phase 2

Mapped issue:

- `OPT-33`

### Phase 4: replace the Sempal-shaped app contract with a generic view/message surface

Goal:

- stop making `AppModel` and `UiAction` the primary public way to use Radiant

Includes:

- generic declarative tree/view surface
- host-defined message emission
- generic runtime bridge path

Dependencies:

- Phase 0
- Phase 1
- Phase 2

Mapped issue:

- `OPT-34`

Status:

- Implemented as the first public `radiant::runtime` surface in
  `vendor/radiant/src/runtime/*`.
- New host applications can now project a generic `UiSurface<Message>` tree
  composed from public layout containers plus reusable `radiant::widgets`.
- `DeclarativeRuntimeBridge` now supports `state -> surface` projection and
  host-defined `message -> state` reduction without depending on
  `AppModel` or `UiAction`.
- `SurfaceRuntime` now provides the generic runtime loop for this surface:
  it runs layout, routes backend-neutral widget input, maps widget outputs to
  host-defined messages, reduces them, and reprojects the next immutable
  surface snapshot.
- The legacy `radiant::app` contract remains available as the migration-time
  compatibility path while native-runtime integration is still finishing its
  transition.
- Public end-to-end coverage now lives in
  `vendor/radiant/tests/runtime_surface_public_api.rs`.

### Phase 5: isolate the current native shell as compatibility code

Goal:

- make the Sempal shell explicit as compatibility infrastructure

Includes:

- namespace/module boundary cleanup
- docs and ownership updates
- reduced coupling from generic surfaces into shell-specific code

Dependencies:

- Phase 0
- Phase 4

Mapped issue:

- `OPT-35`

### Phase 6: migrate Sempal slice-by-slice

Goal:

- validate the generic Radiant surfaces against real product UI

Includes:

- one narrow pilot surface first
- additional vertical slices after the pilot proves out the API

Dependencies:

- Phase 3
- Phase 4
- Phase 5

Mapped issue:

- `OPT-36`

## Implementation guardrails

- Keep Sempal shippable at every phase.
- Prefer additive compatibility layers over destructive replacement early.
- Do not expand the current shell-specific `AppModel`/`UiAction` surface further
  unless the work is explicitly compatibility-only.
- New generic UI work should land in public container/widget/runtime surfaces,
  not in `native_shell` helpers.
- New Sempal product work should compose generic Radiant surfaces where
  available instead of creating new shell-only helper structs.

## Risks and pressure points

### Risk: duplicating generic widgets and shell widgets

Mitigation:

- freeze growth of shell-only controls except for compatibility fixes
- route new reusable control work into `widgets`

### Risk: theme/style API remains shell-shaped

Mitigation:

- split generic design tokens from shell/chrome tokens
- keep app-specific styling decisions out of public widget contracts

### Risk: runtime stays coupled to one app-level action enum

Mitigation:

- move to host-defined message types in the generic runtime/bridge surface

### Risk: migration stalls between generic API design and real app adoption

Mitigation:

- use a pilot vertical slice early
- follow the pilot with small, high-signal migrations instead of waiting for a
  full rewrite

## New issue candidates from this plan

This planning pass identifies two follow-up issues not yet represented in the
current milestone backlog:

1. Split generic theming/tokens from Sempal shell styling.
   - Why: current style tokens are still largely shell-specific, and they sit
     on the critical path between layout/widgets and compatibility shell code.

2. Plan and execute post-pilot vertical-slice migrations after the first slice.
   - Why: the current backlog has only one pilot migration issue, but not the
     follow-on lane for migrating additional Sempal surfaces once the pilot
     succeeds.

These should be tracked separately so the pilot does not silently grow into a
full rewrite.
