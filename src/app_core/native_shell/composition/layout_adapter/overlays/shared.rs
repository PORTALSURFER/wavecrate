//! Shared slot-tree helpers for overlay geometry modules.

use crate::gui::layout_core::{
    Constraints, Insets, LayoutNode, SizeModeCross, SizeModeMain, SlotChild, SlotParams,
};
use crate::gui::types::{Point, Rect, Vector2};

/// Build a fill child with a deterministic non-zero fill weight.
pub(super) fn fill_child(node_id: u64, weight: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fill(weight.max(0.0001)),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(0.0, f32::INFINITY, 0.0, f32::INFINITY),
            margin: Insets::default(),
            align_cross_override: None,
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, 1.0)),
    }
}

/// Build a fixed-size child with explicit main/cross constraints.
pub(super) fn fixed_child(
    node_id: u64,
    width: f32,
    height: f32,
    align_cross: crate::gui::layout_core::CrossAlign,
) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(height.max(0.0)),
            size_cross: SizeModeCross::Fixed(width.max(0.0)),
            constraints: Constraints::new(
                width.max(0.0),
                width.max(0.0),
                height.max(0.0),
                height.max(0.0),
            ),
            margin: Insets::default(),
            align_cross_override: Some(align_cross),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(width.max(1.0), height.max(1.0))),
    }
}

/// Build a fixed-width child for horizontal button rows.
pub(super) fn fixed_width_button(node_id: u64, width: f32, left_margin: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(width.max(0.0)),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(width.max(0.0), width.max(0.0), 0.0, f32::INFINITY),
            margin: Insets {
                left: left_margin.max(0.0),
                ..Insets::default()
            },
            align_cross_override: None,
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(width.max(1.0), 1.0)),
    }
}

/// Resolve a rect from layout output or return a fallback.
pub(super) fn rect_for(
    rects: &std::collections::BTreeMap<u64, Rect>,
    id: u64,
    fallback: Rect,
) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

/// Clamp a rect to a containing bounds rect.
pub(super) fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

/// Return an empty rect pinned at the bounds origin.
pub(super) fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

/// Inset horizontally with saturation.
pub(super) fn inset_horizontal(rect: Rect, inset: f32) -> Rect {
    let inset = inset.max(0.0).min((rect.width() * 0.5).max(0.0));
    Rect::from_min_max(
        Point::new(rect.min.x + inset, rect.min.y),
        Point::new(rect.max.x - inset, rect.max.y),
    )
}

/// Inset both axes with saturation.
pub(super) fn inset_uniform(rect: Rect, inset: f32) -> Rect {
    let inset_x = inset.max(0.0).min((rect.width() * 0.5).max(0.0));
    let inset_y = inset.max(0.0).min((rect.height() * 0.5).max(0.0));
    Rect::from_min_max(
        Point::new(rect.min.x + inset_x, rect.min.y + inset_y),
        Point::new(rect.max.x - inset_x, rect.max.y - inset_y),
    )
}
