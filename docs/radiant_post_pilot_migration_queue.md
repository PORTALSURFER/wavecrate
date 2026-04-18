# Radiant Post-Pilot Migration Queue

Status: planned follow-up queue for `OPT-38`

Purpose: define the ordered Sempal UI slices to migrate after the `OPT-36`
status-bar pilot so the generic Radiant adoption lane stays narrow, testable,
and bounded.

## Inputs

- `docs/radiant_library_architecture_plan.md` defines the phase-6
  slice-by-slice migration strategy.
- `docs/radiant_status_bar_pilot.md` records what the first slice proved and
  which gaps remain.
- `docs/gui_migration_parity.md` and the current native-shell module layout show
  the remaining chrome surfaces and their validation hooks.

## Pilot takeaways that shape the queue

The status-bar pilot proved that Sempal can compose one production shell band
from public `radiant::layout`, `radiant::runtime`, and `radiant::widgets`
without rewriting the full compatibility shell.

It also exposed three constraints that should drive the next queue:

1. Keep follow-up slices inside compact chrome bands where generic surfaces can
   own composition and layout without needing to host the full scene graph.
2. Prefer surfaces that reuse the pilot's text/canvas embedding pattern before
   taking on denser interactive controls.
3. Delay the heaviest control strip until generic button, toggle, and text-input
   hosting has already been exercised on smaller surfaces.

## Recommended rollout order

### 1. Top bar chrome and update cluster

Why first:

- closest post-pilot analogue to the footer band
- compact layout with shallow domain coupling
- exercises generic buttons plus the volume meter/update action area without
  touching browser rows or waveform rendering

Validation shape:

- focused native-shell tests for options hit targets, update actions, and volume
  affordances
- compact/standard/wide snapshot coverage for the top bar band
- GUI automation checks for options-open and update-panel actions

### 2. Sidebar chrome bands

Scope:

- sidebar header/footer labels, source controls, and bounded footer actions
- exclude source/folder row virtualization, inline editors, and recovery list
  behavior

Why second:

- remains mostly chrome composition with bounded button surfaces
- reuses the generic button/text paint adapter pattern from the top bar
- keeps the risky row-list migration out of the early queue

Validation shape:

- existing sidebar chrome-layout tests
- focused hit-testing checks for sidebar header/footer controls
- snapshot checks across density tiers

### 3. Waveform header metadata band

Scope:

- loaded-sample title plus tempo/zoom/transport metadata rows
- exclude waveform plot, overlays, selection affordances, and toolbar controls

Why third:

- text-first band with low behavioral risk
- proves generic surface embedding directly adjacent to a still-compat waveform
  plot
- keeps the waveform-specific interaction surface for a later issue

Validation shape:

- waveform-header layout tests
- snapshot checks for loading, loaded, and compact-width states

### 4. Browser tabs and toolbar strip

Scope:

- samples/map tabs, search field, activity label, and sort/status chips
- exclude browser table rows, table virtualization, and row-level interactions

Why fourth:

- first slice that leans heavily on generic interactive widget hosting
- depends on a proven pattern for button, toggle, and text-input embedding
- still avoids the much larger browser-row migration

Validation shape:

- browser-toolbar rendering and hit-testing tests
- browser-search focus/input checks
- GUI automation coverage for browser search and tab switching

### 5. Waveform toolbar control strip

Scope:

- transport button, loop/compare/slice toggles, BPM value field, and related
  toolbar controls
- exclude waveform plot rendering, hover overlays, and edit-handle geometry

Why fifth:

- densest compact control cluster in the shell chrome
- includes modifier-sensitive actions, active/inactive button states, and a BPM
  value input path
- should wait until the smaller text/button/input bands have already landed

Validation shape:

- waveform-toolbar layout and hit-testing tests
- hover-tooltip and BPM-input coverage
- GUI automation coverage for transport, loop, and toolbar button flows

## Issue breakdown

Create one follow-up issue per slice:

1. `OPT-45` — `Sempal: migrate the top bar chrome and update cluster onto generic Radiant surfaces`
2. `OPT-46` — `Sempal: migrate the sidebar header and footer chrome bands onto generic Radiant surfaces`
3. `OPT-47` — `Sempal: migrate the waveform header metadata band onto generic Radiant surfaces`
4. `OPT-48` — `Sempal: migrate the browser tabs and toolbar strip onto generic Radiant surfaces`
5. `OPT-49` — `Sempal: migrate the waveform toolbar control strip onto generic Radiant surfaces`

Each issue should stay narrow enough to:

- land independently behind the compatibility shell
- keep product behavior materially unchanged
- reuse existing native-shell validation hooks instead of inventing a new test
  harness

## Dependency shape

- `OPT-36` blocks the whole post-pilot queue because the pilot defines the first
  adapter pattern and the real missing API edges.
- `OPT-45` is the first direct follow-up and should unblock the rest of the
  queue.
- `OPT-46`, `OPT-47`, and `OPT-48` can all follow `OPT-45` once the generic
  control-hosting pattern is stable.
- `OPT-49` should wait on both `OPT-47` and `OPT-48` because it combines the
  same dense button/input behaviors with a tighter local chrome band.

## Explicitly deferred work

These surfaces should not be folded into the early post-pilot queue:

- browser table rows and virtualization
- source/folder row virtualization and inline editors
- waveform plot rendering, overlays, and edit handles
- map canvas rendering

Those areas are larger behavior migrations, not compact chrome-band migrations,
and should be planned separately after the early queue confirms the generic
surface adapter pattern is stable across multiple shell bands.
