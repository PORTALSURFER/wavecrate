# Radiant Public Widget Model

Status: implemented design contract for `OPT-32`

Purpose: define the first-class public widget taxonomy for `vendor/radiant`
separately from layout containers, while keeping runtime/message migration work
for later issues.

## Why this exists

`radiant::layout` already defines how containers measure and place children.
That is not enough to describe reusable controls.

This document defines the public widget layer that sits:

- above layout containers
- below app-specific screens
- outside Sempal shell-specific model types

The goal is to make future primitive implementation work split cleanly by
widget type without re-opening the container vocabulary every time.

## Containers vs widgets

Containers own child placement.

They decide:

- slot ownership
- spacing and padding
- alignment
- overflow and virtualization

Widgets own user-facing behavior inside the rectangle assigned by layout.

They decide:

- intrinsic sizing
- focus participation
- interaction semantics
- emitted message families
- paint behavior inside the assigned bounds

This boundary stays strict:

- containers do not define button, toggle, text-entry, or row behavior
- widgets do not own general-purpose child layout policy

## Public widget taxonomy

The public taxonomy currently includes:

- `Text`
- `Button`
- `Toggle`
- `TextInput`
- `Scrollbar`
- `ListItem`
- `Canvas`

These map to `radiant::widgets::WidgetKind` and `radiant::widgets::WidgetSpec`.

The taxonomy is intentionally small. It is large enough to express reusable
controls, but narrow enough that follow-up implementation work can be split by
primitive type instead of by app screen.

## Shared widget responsibilities

Every public widget descriptor in `radiant::widgets` carries one shared
contract:

- stable identity via `WidgetId`
- intrinsic sizing via `WidgetSizing`
- focus participation via `FocusBehavior`
- shared visual state via `WidgetState`
- paint expectations via `PaintContract`
- theme-agnostic style vocabulary via `WidgetStyle`
- declared message families via `WidgetMessageKind`

This means each primitive can answer the same core questions:

1. How big does it want to be?
2. Can it receive focus?
3. Which interaction states matter?
4. Does it clip or allow overflow while painting?
5. Which kinds of host messages can it emit?

## Styling and state patterns

The public widget model avoids Sempal shell naming.

Instead it exposes generic shared vocabularies:

- `WidgetState` for hovered, pressed, focused, selected, active, disabled, and
  read-only behavior
- `WidgetTone` for semantic tone
- `WidgetProminence` for chrome weight
- `WidgetStyle` for the minimal shared styling contract

This keeps primitive state semantics reusable while leaving theme-token binding
to later theming work.

## Message model boundary

The widget layer declares message families, not a concrete app action enum.

Current public families:

- `Activate`
- `ValueChanged`
- `TextEdited`
- `ScrollRequested`
- `ItemInvoked`
- `CanvasInput`

This is deliberate. `OPT-32` defines the widget vocabulary and behavior
contracts. Binding those families to host-defined message payloads belongs to
the generic runtime/view work tracked separately by `OPT-34`.

## Composition with public containers

Widgets currently compose with public containers by projecting to public
layout leaves using `WidgetCommon::layout_node()` or `WidgetSpec::layout_node()`.

That keeps the current ownership boundary explicit:

- containers still own placement through `radiant::layout`
- widgets supply the leaf behavior contract plus intrinsic size

Example:

```rust
use radiant::{
    layout::{
        ContainerKind, ContainerPolicy, LayoutNode, Point, Rect, SlotChild, SlotParams, Vector2,
        layout_tree,
    },
    widgets::{ButtonWidget, TextWidget, WidgetSizing},
};

let title = TextWidget::new(
    10,
    "Sources",
    WidgetSizing::fixed(Vector2::new(72.0, 20.0)).with_baseline(14.0),
);
let import = ButtonWidget::new(
    11,
    "Import",
    WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
);

let row = LayoutNode::container(
    1,
    ContainerPolicy {
        kind: ContainerKind::Row,
        spacing: 8.0,
        ..ContainerPolicy::default()
    },
    vec![
        SlotChild::new(SlotParams::fill(), title.common.layout_node()),
        SlotChild::new(SlotParams::fill(), import.common.layout_node()),
    ],
);

let output = layout_tree(
    &row,
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 32.0)),
);

assert!(output.rects.contains_key(&10));
assert!(output.rects.contains_key(&11));
```

## Primitive follow-up split

This contract is intended to make follow-up implementation work separable into
clean lanes:

- button/toggle
- text input
- scrollbar
- list item
- canvas/custom surface
- richer text behavior

That split is one of the main acceptance goals for `OPT-32`.
