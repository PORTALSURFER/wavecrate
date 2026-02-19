# Radiant Strict Hierarchical Slot-Based Layout Spec

This document defines the required layout/tree design for `vendor/radiant`.
It is the source-of-truth contract for layout behavior in the native shell
runtime and related UI surfaces.

## 0. Goals

- Deterministic, repeatable layout results given the same tree and root frame.
- Fully responsive behavior when root size changes.
- Composition-first model where containers own layout and widgets express
  preferences.
- Explicit, predictable handling of overflow and underflow.
- Runtime performance suitable for real-time UI with caching and minimal
  recompute.

Non-goals:

- Freeform drag placement inside the layout engine itself (can be implemented
  as a widget).
- General 2D global constraint solving across arbitrary relationships.

## 1. Core Concepts

### 1.1 Node Types

All UI elements are nodes in a tree:

- `WidgetNode`: leaf (or mostly-leaf) content node with optional intrinsic size.
- `ContainerNode`: node that owns slots and executes a layout algorithm.
- `Slot`: child attachment point owned by a container, including child layout
  parameters.

Rules:

- Every non-root node is owned by exactly one slot.
- Child layout parameters are parent-owned slot data, not child-owned state.

### 1.2 Coordinate Space

- Layout is computed in parent-local coordinates.
- Output for each node is `Rect { x, y, w, h }`.
- Nodes do not assign global positions; containers place children.

## 2. Layout Data Model

### 2.1 Sizes

Supported size modes:

- `Fixed(px)`
- `Fill(weight)`
- `Percent(p)` (optional)
- `Intrinsic`
- `Aspect(ratio)` (optional)

Supported constraints:

- `min_w`, `max_w`, `min_h`, `max_h` (`max` may be infinity)
- `padding` on containers
- `margin` on slots

### 2.2 Alignment

- Main-axis: `Start | Center | End | SpaceBetween | SpaceAround | SpaceEvenly`
- Cross-axis: `Start | Center | End | Stretch`

### 2.3 Overflow Policy (mandatory)

Every container must define one explicit overflow policy:

- `Clip` (default for most containers)
- `Scroll`
- `Wrap`
- `Shrink` or `Compress`

Widget-level text behavior may add:

- `Ellipsize`

No implicit fallback behavior is allowed.

## 3. Deterministic Layout Algorithm

### 3.1 Two-pass measure/layout

Use a two-pass algorithm.

Pass A: measure (bottom-up)

- Signature: `measure(node, constraints) -> Size`
- Widgets compute preferred size from content plus constraints.
- Containers measure children from slot rules, then compute preferred size.
- Measurement must not depend on final absolute position.

Pass B: layout (top-down)

- Signature: `layout(node, rect) -> placements`
- Container receives final rect and assigns child rects inside content area.
- Widgets accept rects and do not reposition themselves.

### 3.2 Constraint propagation

- Constraints flow downward from parent content bounds after padding/margins.
- All computed sizes must be clamped to constraints.
- Intrinsic sizes are also clamped.

### 3.3 Space distribution (Row/Column/Flex-like)

Containers must apply this order:

1. Compute available main-axis space after padding and margins.
2. Resolve non-flex children: `Fixed`, `Intrinsic`, and optionally `Percent`.
3. Compute remaining space.
4. Distribute remaining space across `Fill(weight)` children by weight.
5. Apply min/max clamping.
6. If clamping changes totals, rerun flex distribution with clamped children
   removed from the flex pool.
7. If overflow remains, apply overflow policy.

### 3.4 Compression policy (mandatory)

When `sum(min_sizes) > available_space`, compress deterministically:

1. Reduce `Fill` children to min.
2. Reduce `Intrinsic` children to min.
3. Reduce `Fixed` children to min only when slot allows fixed compression.
4. If still not fitting, apply overflow policy (`Clip` or `Scroll`).

This order is required and must not vary by frame or platform.

## 4. Slot Semantics

### 4.1 Slot ownership

- Containers define fixed slot sets or deterministic slot lists.
- Children cannot exist without slots.
- Slot parameters are parent-owned.

### 4.2 Required slot parameters

Each slot must define:

- `size_main: Fixed | Fill(weight) | Intrinsic | Percent`
- `size_cross: Fixed | Fill | Intrinsic`
- `min/max` constraints
- `margin`
- optional alignment overrides

### 4.3 Single-child and multi-child containers

- Single-slot containers: `PaddingBox`, `AlignBox`, `AspectBox`.
- Multi-slot containers: `Row`, `Column`, `Grid`, `Stack`.
- Dynamic child lists are still slot lists with deterministic slot ordering.

## 5. Baseline Container Set

Implement and maintain the following baseline primitives:

- `PaddingBox`: one slot; applies padding.
- `AlignBox`: one slot; positions child within the assigned rect.
- `Row` and `Column`: multi-slot flex distribution plus spacing/alignment.
- `Grid`: row/column tracks, gaps, per-cell slot behavior.
- `Stack`: child overlays in shared rect; z-order from slot order.
- `ScrollView`: one-slot viewport plus scrolling.
- `Wrap` or `Flow` (optional): defined wrapping policy.

Each container must document:

- measure behavior
- layout behavior
- overflow behavior
- deterministic ordering guarantees

## 6. Widget Contract

### 6.1 Intrinsic size

Widgets may expose:

- `min_intrinsic(constraints)`
- `preferred(constraints)`
- optional `baseline` for text alignment

Text widgets must define:

- wrapping mode
- ellipsis mode
- line-height policy

### 6.2 Rendering

- Widgets render into final assigned rect.
- Drawing outside rect is allowed only when overflow policy permits it.

## 7. Responsiveness Requirements

- Sizes must be relative or constraint-clamped.
- DPI scaling must be applied from one root scaling point.
- Min/max constraints must prevent extreme stretch/collapse.
- Optional `SwitchLayout` container may select explicit subtrees by root
  size breakpoints.

## 8. Determinism and Stability Rules

Layout output may depend only on:

- tree structure
- slot parameters
- widget intrinsic sizes
- root rect and constraints

Hard requirements:

- No randomness.
- No frame-time dependence.
- Iterative flex resolution must have deterministic termination.
- Floating-point to pixel conversion must use one global rounding strategy.
- Z-order follows slot order unless an explicit and deterministic z rule is
  documented.

## 9. Invalid States and Error Handling

Required behavior:

- Negative size after constraints: clamp to `0` and emit diagnostic.
- Contradictory constraints (`min > max`): clamp `max = min` and emit
  diagnostic.
- Missing overflow policy: default to `Clip` and emit diagnostic.
- Cycles in layout tree: reject at construction.

Debug mode must be able to render:

- container bounds
- padding and margin outlines
- slot outlines
- overflow indicators
- measured vs final size overlays

## 10. Performance and Caching

### 10.1 Dirty propagation

Track and propagate:

- `layout_dirty`: geometry-affecting changes
- `measure_dirty`: intrinsic-affecting changes

Only recompute affected subtrees.

### 10.2 Measure cache keys

Cache measure results by:

- node id
- constraint key
- relevant widget state version

### 10.3 Allocation and traversal

- Layout and paint traversal must avoid excessive allocation.
- Prefer arena/id-based structures for stable, high-frequency layout passes.

## 11. Minimal API Surface

- `NodeId`
- `Widget` trait: `measure(constraints)`, `paint(rect, ctx)`
- `Container` trait: `measure(children, constraints)`, `layout(children, rect)`
- `SlotParams`
- `Constraints { min_w, max_w, min_h, max_h }`
- `LayoutResult { rects, overflow_flags, diagnostics }`
- `LayoutState { scroll_offsets }` for stateful containers (`ScrollView`)
- `LayoutDebugOptions` and `LayoutDebugPrimitive` for non-interactive debug overlays
- `LayoutDiagnosticCode` for stable diagnostic categories

## 12. Test Requirements

### 12.1 Golden layout tests

For each container, assert exact output rects across:

- multiple root sizes
- mixed fixed/fill/intrinsic children
- min/max constraints
- overflow scenarios

### 12.2 Property tests

- Child rects remain inside parent content bounds unless policy allows.
- No NaN values.
- No negative widths or heights.
- Determinism: identical inputs produce identical outputs.

### 12.3 Stress tests

- Deep nesting (`100` to `500` levels).
- Large slot lists (`1k` to `10k` items), especially for scroll/virtualized
  containers.

## Current Implementation Status (Phase 2)

- `LayoutEngine::layout_with_state(...)` is implemented to accept `LayoutState`
  and `LayoutDebugOptions`.
- `ScrollView` now clamps requested offsets to viewport-valid ranges and emits
  `InvalidScrollOffsetClamped` diagnostics when clamping occurs.
- Layout output now includes debug primitives (node bounds, content bounds,
  slot margins, overflow markers) when debug options are enabled.
- Diagnostics now carry stable codes (`LayoutDiagnosticCode`) so tests and
  tooling can assert behavior without relying on free-form text.
- Virtualized scroll/windowing behavior remains intentionally out of scope for
  this phase.

## Current Implementation Status (Phase 3)

- `ScrollView` now supports optional linear virtualization for `Row` and
  `Column` content via `ContainerPolicy::virtualization`.
- Virtualization emits stable metadata and diagnostics:
  - `LayoutOutput.virtual_windows`
  - `LayoutOutput.stats`
  - `VirtualizationPolicyIgnored` and `VirtualizationWindowClamped`
- Debug overlays now include virtualization primitives:
  - `ViewportBounds`
  - `VirtualWindowBounds`
  - `CulledRegion`
- `10k`-item stress coverage now asserts bounded materialization counts for a
  virtualized scroll list.
- Wrap/grid/general subtree virtualization remains out of scope; unsupported
  cases default to non-virtualized layout with a diagnostic.

## Current Implementation Status (Phase 4)

- Virtualized linear layout now resolves full `Row`/`Column` sizing semantics
  before windowing:
  - `Fixed`
  - `Intrinsic`
  - `Percent`
  - `Fill(weight)`
- Virtualization no longer requires `MainAlign::Start`; resolved linear
  alignment is preserved in windowed layout.
- Virtualization cache invalidation now targets dependency subtrees instead of
  clearing the entire cache on every dirty mark.
- Virtualization metadata now records resolved window and alignment details:
  - `window_main_start`
  - `window_main_end`
  - `resolved_total_main`
  - `alignment_mode`
- Additional fallback diagnostics now classify span/alignment fallback paths:
  - `VirtualizationAlignmentFallback`
  - `VirtualizationSpanResolutionFallback`

### Virtualization Support Matrix

| Container | Axis | Main Size Modes | Main Align Modes | Behavior |
| --- | --- | --- | --- | --- |
| `Row` | Horizontal | Fixed, Intrinsic, Percent, Fill | Start, Center, End, SpaceBetween, SpaceAround, SpaceEvenly | Virtualized |
| `Column` | Vertical | Fixed, Intrinsic, Percent, Fill | Start, Center, End, SpaceBetween, SpaceAround, SpaceEvenly | Virtualized |
| `Wrap` | Horizontal/Vertical | N/A | N/A | Fallback + diagnostic |
| `Grid` | Horizontal/Vertical | N/A | N/A | Fallback + diagnostic |
| non-linear containers | N/A | N/A | N/A | Fallback + diagnostic |

## Current Implementation Status (Phase 5)

- Debug overlays now support measured-vs-final inspection via
  `DebugPrimitiveKind::MeasuredBounds` and `LayoutDebugOptions::show_measured`.
- `LayoutEngine` now exposes subtree dirty APIs:
  - `mark_layout_dirty_subtree(root, node_id)`
  - `mark_measure_dirty_subtree(root, node_id)`
- Dirty subtree marking includes ancestor path nodes so parent/container measure
  cache entries are invalidated deterministically for affected branches.
- Native-shell top-bar control geometry now resolves through layout-core slot
  trees (`layout_adapter::compute_top_bar_controls_sections`) instead of
  standalone ad-hoc rectangle math in shell state.
- Rounding contract remains explicit and unchanged:
  - origin: `floor(x), floor(y)`
  - size: `round(w), round(h)`
  - size lower bound: `0`

## Current Implementation Status (Phase 6)

- Native-shell high-impact layout bands now resolve through layout-core slot
  trees in `layout_adapter`:
  - top-bar banding (`title row`, `controls row`, `title cluster`,
    `action cluster`)
  - browser banding (`tabs`, `toolbar`, `table header`, `rows`, `footer`)
  - sidebar banding (`header`, `rows`, `footer`)
- Sidebar source/folder section partitioning now routes through
  `layout_adapter::compute_sidebar_row_sections(...)`, replacing shell-state
  local rectangle partition helpers.
- `ShellLayout::build_with_style(...)` now consumes the slotized band outputs
  from `layout_adapter` for top-bar/browser/sidebar geometry instead of local
  ad-hoc panel arithmetic.

## Current Implementation Status (Phase 7)

- Native-shell overlay geometry now routes through slotized layout adapters:
  - `layout_adapter::compute_prompt_overlay_sections(...)`
  - `layout_adapter::compute_progress_overlay_sections(...)`
  - `layout_adapter::compute_drag_overlay_rect(...)`
- Browser/sidebar/top-bar component micro-layout now routes through slotized
  control adapters:
  - top-bar update action buttons
  - browser action strip buttons
  - sidebar footer action buttons
  - browser toolbar search/activity/sort partitions
- Shell-state rendering and hit-testing now consume adapter-owned overlay/button
  sections instead of local ad-hoc rect math for those surfaces.

## Current Implementation Status (Phase 8)

- Sidebar folder-header micro-layout now routes through slotized adapter
  helpers in `layout_adapter::sidebar_header`:
  - `compute_sidebar_folder_header_layout(...)` for title row, metadata row,
    and recovery badge geometry/label state
  - `compute_source_section_divider_rect(...)` for source/folder divider
    placement
- Native-shell sidebar rendering and related layout tests now consume the
  adapter outputs for header text/badge/divider placement.
- Legacy shell-state local helper arithmetic for those sidebar surfaces has
  been removed in favor of adapter-owned deterministic layout contracts.

## Current Implementation Status (Phase 9)

- Status-bar segment geometry now routes through slotized adapter helpers:
  - `layout_adapter::compute_status_bar_segments(...)` for left/center/right
    segment partitioning
- Status text-line geometry now routes through slotized adapter helpers:
  - `layout_adapter::compute_status_text_line_rect(...)` for per-segment
    text-line placement bounds
- `ShellLayout::build_with_style(...)` and native-shell status rendering now
  consume adapter-owned status geometry instead of local proportional/status
  text rect arithmetic.

## Current Implementation Status (Phase 10)

- Waveform header text-row geometry now routes through slotized adapter
  helpers:
  - `layout_adapter::compute_waveform_header_text_layout(...)` for header
    title and metadata row bounds
- Native-shell waveform header text rendering now consumes adapter-owned
  title/metadata row rects instead of local text-row offset arithmetic.

## Current Implementation Status (Phase 11)

- Browser list text/chip geometry now routes through slotized adapter helpers:
  - `layout_adapter::compute_browser_header_text_layout(...)` for list
    table-header `#`/`Sample`/`Bucket` label bounds
  - `layout_adapter::compute_browser_row_text_layout(...)` for per-row
    index/sample label bounds plus bucket chip/label bounds
- Native-shell browser list rendering and focused-row overlay rendering now
  consume adapter-owned browser row/header text/chip geometry instead of local
  browser text baseline and inset arithmetic.

## Current Implementation Status (Phase 12)

- Map-active browser-header metadata geometry now routes through slotized
  adapter helpers:
  - `layout_adapter::compute_browser_map_header_text_layout(...)` for
    map-header left/right metadata label bounds
- Native-shell map-active browser-header rendering now consumes adapter-owned
  map-header label geometry instead of local text baseline and right-anchor
  rect arithmetic.

## Current Implementation Status (Phase 13)

- Top-bar update copy geometry now routes through slotized adapter helpers:
  - `layout_adapter::compute_top_bar_update_text_layout(...)` for update
    status/controls text-line bounds in the action cluster
- Native-shell top-bar update text rendering now consumes adapter-owned
  line geometry instead of local reserved-width and baseline arithmetic.

## Current Implementation Status (Phase 14)

- Prompt/progress/drag overlay copy geometry now routes through slotized
  adapter helpers:
  - `layout_adapter::compute_prompt_overlay_text_layout(...)` for prompt
    title/message/target/input/error/button-label text-line bounds
  - `layout_adapter::compute_progress_overlay_text_layout(...)` for progress
    title/detail/counter/cancel-label text-line bounds
  - `layout_adapter::compute_drag_overlay_text_layout(...)` for drag-banner
    label text-line bounds
- Native-shell overlay rendering now consumes adapter-owned text-line geometry
  instead of local y-offset and button text-top arithmetic for those paths.

## Current Implementation Status (Phase 15)

- Browser chrome tabs/toolbar/footer copy geometry now routes through slotized
  adapter helpers:
  - `layout_adapter::compute_browser_tabs_text_layout(...)` for tab-label
    text-line bounds
  - `layout_adapter::compute_browser_toolbar_text_layout(...)` for search/
    activity/sort label bounds
  - `layout_adapter::compute_browser_footer_text_rect(...)` for browser-footer
    summary text-line bounds
- Native-shell browser chrome rendering now consumes adapter-owned text-line
  geometry instead of local tab/toolbar/footer text baseline arithmetic.

## Current Native-Shell Gap (tracked)

Remaining native-shell layout work is now concentrated in render-time text and
annotation micro-placement outside sidebar/status/waveform-header/browser-list
/map-header/top-bar-update/overlay-copy/browser-chrome paths (for example
map-canvas annotations and other residual text/button paths) that still uses
local rect arithmetic and should be migrated into slotized text/layout adapters
in a follow-up phase.
