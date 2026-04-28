use super::*;

/// Width in logical pixels for hovered waveform resize-edge highlights.
const RESIZE_EDGE_HIGHLIGHT_WIDTH: f32 = 4.0;
/// Fraction of waveform height used by centered waveform resize-edge targets.
const RESIZE_EDGE_HEIGHT_RATIO: f32 = 0.34;
/// Height in logical pixels for loop-range marker bars.
const LOOP_BAR_HEIGHT: f32 = 3.0;
/// Width/height in logical pixels for the playback-selection drag handle.
const SELECTION_DRAG_HANDLE_SIZE: f32 = 12.0;
/// Width in logical pixels for bottom-center selection shift handles.
const SELECTION_SHIFT_HANDLE_WIDTH: f32 = 14.0;
/// Height in logical pixels for bottom-center selection shift handles.
const SELECTION_SHIFT_HANDLE_HEIGHT: f32 = 7.0;

/// Emit a hovered playback-selection resize edge accent.
pub(super) fn emit_hovered_selection_resize_edge(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    selection_rect: Rect,
    accent_color: Rgba8,
    hovered_resize_edge: Option<WaveformResizeHoverEdge>,
) {
    let left_edge = match hovered_resize_edge {
        Some(WaveformResizeHoverEdge::SelectionStart) => Some(true),
        Some(WaveformResizeHoverEdge::SelectionEnd) => Some(false),
        _ => None,
    };
    if let Some(left_edge) = left_edge {
        emit_resize_edge_highlight(primitives, style, selection_rect, left_edge, accent_color);
    }
}

/// Emit a hovered edit-selection resize edge accent.
pub(super) fn emit_hovered_edit_resize_edge(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    edit_selection_rect: Rect,
    accent_blue: Rgba8,
    hovered_resize_edge: Option<WaveformResizeHoverEdge>,
) {
    let left_edge = match hovered_resize_edge {
        Some(WaveformResizeHoverEdge::EditSelectionStart) => Some(true),
        Some(WaveformResizeHoverEdge::EditSelectionEnd) => Some(false),
        _ => None,
    };
    if let Some(left_edge) = left_edge {
        emit_resize_edge_highlight(
            primitives,
            style,
            edit_selection_rect,
            left_edge,
            accent_blue,
        );
    }
}

/// Emit the playback-selection drag handle used for clip export drags.
pub(super) fn emit_selection_drag_handle(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    selection_rect: Rect,
) {
    let handle = selection_drag_handle_rect(selection_rect);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: handle,
            color: translucent_overlay_color(style.surface_overlay, style.accent_warning, 0.92),
        }),
    );
    push_border(
        primitives,
        handle,
        blend_color(style.accent_warning, style.text_primary, 0.55),
        style.sizing.border_width,
    );
}

/// Emit the bottom-center handle used to slide one selection along the waveform.
pub(super) fn emit_selection_shift_handle(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    selection_rect: Rect,
    accent_color: Rgba8,
) {
    let handle = selection_shift_handle_rect(selection_rect);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: handle,
            color: translucent_overlay_color(style.surface_overlay, accent_color, 0.82),
        }),
    );
    push_border(
        primitives,
        handle,
        blend_color(accent_color, style.text_primary, 0.48),
        style.sizing.border_width,
    );
}

/// Emit top/bottom loop-range bars over the active playback selection.
pub(super) fn emit_waveform_loop_bar(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    selection: Rect,
) {
    let bar_height = LOOP_BAR_HEIGHT
        .max(style.sizing.border_width)
        .min(selection.height().max(1.0));
    let top = Rect::from_min_max(
        selection.min,
        Point::new(
            selection.max.x,
            (selection.min.y + bar_height).min(selection.max.y),
        ),
    );
    let bottom = Rect::from_min_max(
        Point::new(
            selection.min.x,
            (selection.max.y - bar_height).max(selection.min.y),
        ),
        selection.max,
    );
    let edge_color = blend_color(style.accent_copper, style.text_primary, 0.2);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: top,
            color: translucent_overlay_color(style.surface_overlay, style.accent_copper, 0.42),
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: bottom,
            color: translucent_overlay_color(style.surface_overlay, style.accent_copper, 0.32),
        }),
    );
    push_border(primitives, top, edge_color, style.sizing.border_width);
    push_border(primitives, bottom, edge_color, style.sizing.border_width);
}

/// Emit one centered resize-edge highlight over the hovered selection boundary.
fn emit_resize_edge_highlight(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    selection_rect: Rect,
    left_edge: bool,
    accent_color: Rgba8,
) {
    let width = RESIZE_EDGE_HIGHLIGHT_WIDTH
        .max(style.sizing.border_width)
        .max(1.0);
    let edge_x = if left_edge {
        selection_rect.min.x
    } else {
        selection_rect.max.x
    };
    let half = width * 0.5;
    let left = (edge_x - half).max(selection_rect.min.x);
    let right = (left + width).min(selection_rect.max.x).max(left + 1.0);
    let handle = centered_resize_edge_rect(selection_rect, left, right, RESIZE_EDGE_HEIGHT_RATIO);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: handle,
            color: translucent_overlay_color(style.surface_overlay, accent_color, 0.72),
        }),
    );
    push_border(
        primitives,
        handle,
        blend_color(accent_color, style.text_primary, 0.62),
        style.sizing.border_width,
    );
}

/// Return the bottom-right playback-selection drag handle rectangle.
fn selection_drag_handle_rect(selection_rect: Rect) -> Rect {
    let size = SELECTION_DRAG_HANDLE_SIZE
        .min(selection_rect.width().max(1.0))
        .min(selection_rect.height().max(1.0));
    Rect::from_min_max(
        Point::new(selection_rect.max.x - size, selection_rect.max.y - size),
        selection_rect.max,
    )
}

/// Return the bottom-center selection shift handle rectangle.
fn selection_shift_handle_rect(selection_rect: Rect) -> Rect {
    let width = SELECTION_SHIFT_HANDLE_WIDTH.min(selection_rect.width().max(1.0));
    let height = SELECTION_SHIFT_HANDLE_HEIGHT.min(selection_rect.height().max(1.0));
    let left = selection_rect.min.x + ((selection_rect.width() - width) * 0.5);
    let top = (selection_rect.max.y - height).max(selection_rect.min.y);
    Rect::from_min_max(
        Point::new(left, top),
        Point::new(
            (left + width).min(selection_rect.max.x),
            selection_rect.max.y,
        ),
    )
}

/// Return a resize-edge rect centered vertically within a selection band.
fn centered_resize_edge_rect(
    selection_rect: Rect,
    left: f32,
    right: f32,
    height_ratio: f32,
) -> Rect {
    let height = (selection_rect.height() * height_ratio.clamp(0.0, 1.0))
        .max(1.0)
        .min(selection_rect.height());
    let center_y = selection_rect.min.y + (selection_rect.height() * 0.5);
    let top = (center_y - (height * 0.5)).max(selection_rect.min.y);
    let bottom = (top + height).min(selection_rect.max.y).max(top + 1.0);
    Rect::from_min_max(Point::new(left, top), Point::new(right, bottom))
}
