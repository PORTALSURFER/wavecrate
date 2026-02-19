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

## Current Native-Shell Gap (tracked)

Current native shell rendering includes a top-bar volume meter that is
hardcoded and non-interactive (a visual placeholder). This is intentionally
out-of-spec for this document and should be replaced with a slot-driven,
model-backed widget that participates in the layout/action contract.
