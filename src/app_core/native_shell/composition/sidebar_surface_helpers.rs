//! Shared widget, slot, and clamp helpers for sidebar chrome surfaces.

use crate::gui::native_shell::style::SizingTokens;
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{Constraints, CrossAlign, Insets, SizeModeCross, SizeModeMain, SlotParams},
    runtime::{SurfaceNode, WidgetMessageMapper},
    widgets::{ButtonWidget, TextWidget, WidgetSizing, WidgetSpec},
};

/// Return the canonical square sidebar-header add-button edge length.
pub(super) fn header_button_side(sizing: SizingTokens) -> f32 {
    (sizing.font_header + (sizing.text_inset_y * 1.5))
        .round()
        .max(12.0)
}

/// Return the bounded per-button footer width for the current sidebar band.
pub(super) fn footer_action_button_width(
    footer_width: f32,
    sizing: SizingTokens,
    button_count: usize,
) -> f32 {
    if button_count == 0 {
        return 0.0;
    }
    let gap = sizing.sidebar_action_button_gap.max(0.0);
    let available_width = (footer_width - (sizing.text_inset_x * 2.0)).max(0.0);
    ((available_width - (gap * button_count.saturating_sub(1) as f32)).max(0.0)
        / button_count as f32)
        .min(sizing.sidebar_action_button_width)
        .max(1.0)
}

/// Build one fixed-height text widget node for a generic sidebar surface.
pub(super) fn text_widget(id: u64, text: &str, width: f32, height: f32) -> SurfaceNode<()> {
    SurfaceNode::widget(
        WidgetSpec::Text(TextWidget::new(
            id,
            text,
            WidgetSizing::fixed(Vector2::new(width.max(1.0), height.max(1.0)))
                .with_baseline((height * 0.75).max(0.0)),
        )),
        WidgetMessageMapper::None,
    )
}

/// Build one fixed-size button widget node for a generic sidebar surface.
pub(super) fn button_widget(id: u64, label: &str, width: f32, height: f32) -> SurfaceNode<()> {
    SurfaceNode::widget(
        WidgetSpec::Button(ButtonWidget::new(
            id,
            label,
            WidgetSizing::fixed(Vector2::new(width.max(1.0), height.max(1.0))),
        )),
        WidgetMessageMapper::None,
    )
}

/// Return a fixed-width slot that stretches vertically inside its parent row.
pub(super) fn fixed_slot(width: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Fixed(width.max(0.0)),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(width.max(0.0), width.max(0.0), 0.0, f32::INFINITY),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Stretch),
        allow_fixed_compress: false,
    }
}

/// Return a fixed-height slot that stretches horizontally inside its parent column.
pub(super) fn fixed_slot_cross_fill(height: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Fixed(height.max(0.0)),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(0.0, f32::INFINITY, height.max(0.0), height.max(0.0)),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Stretch),
        allow_fixed_compress: false,
    }
}

/// Return a fixed-size slot used for square chrome buttons.
pub(super) fn fixed_slot_with_cross(width: f32, height: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Fixed(width.max(0.0)),
        size_cross: SizeModeCross::Fixed(height.max(0.0)),
        constraints: Constraints::new(
            width.max(0.0),
            width.max(0.0),
            height.max(0.0),
            height.max(0.0),
        ),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Center),
        allow_fixed_compress: false,
    }
}

/// Clamp one resolved layout rect back inside the original surface bounds.
pub(super) fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

/// Look up one resolved rect by node id or return the provided fallback.
pub(super) fn rect_for(
    rects: &std::collections::BTreeMap<u64, Rect>,
    id: u64,
    fallback: Rect,
) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}
