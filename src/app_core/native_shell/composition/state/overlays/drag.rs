//! Drag-overlay rendering for the native shell.

use super::*;

const DRAG_CHIP_OFFSET_X: f32 = 18.0;
const DRAG_CHIP_OFFSET_Y: f32 = 14.0;
const DRAG_CHIP_MIN_WIDTH: f32 = 56.0;
const DRAG_CHIP_MAX_WIDTH: f32 = 320.0;
const DRAG_CHIP_MIN_HEIGHT: f32 = 18.0;

/// Render drag feedback overlays for the active drag gesture.
///
/// This keeps the existing bottom banner for destination/status feedback and
/// adds a compact cursor-following chip that shows the dragged payload label.
pub(in crate::gui::native_shell::state) fn render_drag_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !model.drag_overlay.active {
        return;
    }
    render_drag_banner(primitives, text_runs, layout, style, model);
    render_drag_chip(primitives, text_runs, layout, style, model);
}

fn render_drag_banner(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    let sizing = style.sizing;
    let rect = drag_overlay_rect(layout, style);
    let drag_text_layout = compute_drag_overlay_text_layout(rect, sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        rect,
        if model.drag_overlay.valid_target {
            style.accent_mint
        } else {
            style.accent_warning
        },
        sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: drag_banner_text(model),
            position: drag_text_layout.label.min,
            font_size: sizing.font_meta,
            color: if model.drag_overlay.valid_target {
                style.text_primary
            } else {
                style.accent_warning
            },
            max_width: Some(drag_text_layout.label.width().max(24.0)),
            align: TextAlign::Center,
        },
    );
}

fn render_drag_chip(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    let Some(rect) = drag_chip_rect(layout, style, model) else {
        return;
    };
    let sizing = style.sizing;
    let fill = if model.drag_overlay.valid_target {
        translucent_overlay_color(style.surface_overlay, style.accent_mint, 0.78)
    } else {
        translucent_overlay_color(style.surface_overlay, style.accent_warning, 0.72)
    };
    let border = if model.drag_overlay.valid_target {
        blend_color(style.accent_mint, style.text_primary, 0.24)
    } else {
        blend_color(style.accent_warning, style.text_primary, 0.16)
    };
    let text_color = if model.drag_overlay.valid_target {
        style.text_primary
    } else {
        style.accent_warning
    };
    let text_padding_x = drag_chip_padding_x(sizing);
    let text_origin = Point::new(
        rect.min.x + text_padding_x,
        rect.min.y + ((rect.height() - sizing.font_meta) * 0.5).floor(),
    );
    emit_primitive(primitives, Primitive::Rect(FillRect { rect, color: fill }));
    push_border(primitives, rect, border, sizing.border_width);
    emit_text(
        text_runs,
        TextRun {
            text: model.drag_overlay.label.clone(),
            position: text_origin,
            font_size: sizing.font_meta,
            color: text_color,
            max_width: Some((rect.width() - (text_padding_x * 2.0)).max(24.0)),
            align: TextAlign::Left,
        },
    );
}

fn drag_banner_text(model: &AppModel) -> String {
    if model.drag_overlay.target_label.is_empty() {
        model.drag_overlay.label.clone()
    } else {
        format!(
            "{} -> {}",
            model.drag_overlay.label, model.drag_overlay.target_label
        )
    }
}

/// Resolve the floating drag-chip rect for the current pointer anchor.
pub(in crate::gui::native_shell::state) fn drag_chip_rect(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Option<Rect> {
    if !model.drag_overlay.active || model.drag_overlay.label.trim().is_empty() {
        return None;
    }
    let (Some(pointer_x), Some(pointer_y)) =
        (model.drag_overlay.pointer_x, model.drag_overlay.pointer_y)
    else {
        return None;
    };
    let bounds = drag_chip_bounds(layout);
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return None;
    }
    let sizing = style.sizing;
    let padding_x = drag_chip_padding_x(sizing);
    let padding_y = drag_chip_padding_y(sizing);
    let label_width = drag_chip_label_width(&model.drag_overlay.label, sizing.font_meta);
    let desired_width =
        (label_width + (padding_x * 2.0)).clamp(DRAG_CHIP_MIN_WIDTH, DRAG_CHIP_MAX_WIDTH);
    let width = desired_width.min(bounds.width()).max(1.0);
    let desired_height = (sizing.font_meta + (padding_y * 2.0)).max(DRAG_CHIP_MIN_HEIGHT);
    let height = desired_height.min(bounds.height()).max(1.0);
    let pointer = Point::new(f32::from(pointer_x), f32::from(pointer_y));
    let min_x = flip_and_clamp_drag_chip_axis(
        pointer.x,
        width,
        bounds.min.x,
        bounds.max.x,
        DRAG_CHIP_OFFSET_X,
    );
    let min_y = flip_and_clamp_drag_chip_axis(
        pointer.y,
        height,
        bounds.min.y,
        bounds.max.y,
        DRAG_CHIP_OFFSET_Y,
    );
    Some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + width, min_y + height),
    ))
}

fn drag_chip_bounds(layout: &ShellLayout) -> Rect {
    Rect::from_min_max(
        Point::new(layout.root.rect.min.x, layout.top_bar.max.y),
        Point::new(layout.root.rect.max.x, layout.status_bar.min.y),
    )
}

fn drag_chip_label_width(text: &str, font_size: f32) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ((text.chars().count() as f32) * (font_size * 0.56).max(1.0)).ceil()
}

fn drag_chip_padding_x(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.max(6.0)
}

fn drag_chip_padding_y(sizing: SizingTokens) -> f32 {
    sizing.text_inset_y.max(4.0)
}

fn flip_and_clamp_drag_chip_axis(
    pointer: f32,
    extent: f32,
    min_bound: f32,
    max_bound: f32,
    offset: f32,
) -> f32 {
    let mut min = pointer + offset;
    if min + extent > max_bound {
        min = pointer - offset - extent;
    }
    min.clamp(min_bound, (max_bound - extent).max(min_bound))
}
