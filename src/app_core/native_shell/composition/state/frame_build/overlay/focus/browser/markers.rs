use super::*;

pub(super) fn render_browser_row_markers(
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
    model: &AppModel,
    row_border_stroke: f32,
) {
    emit_processing_marker(primitives, row, style);
    emit_playback_age_marker(primitives, row, style, model);
    emit_locked_marker(primitives, row, style, row_border_stroke);
}

fn emit_processing_marker(
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
) {
    let Some(color) = browser_processing_marker_color(style, row.processing_state) else {
        return;
    };
    let marker_width = (style.sizing.border_width * 3.0).clamp(2.0, 5.0);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(
                row.rect.min,
                Point::new(
                    (row.rect.min.x + marker_width).min(row.rect.max.x),
                    row.rect.max.y,
                ),
            ),
            color,
        }),
    );
}

fn emit_playback_age_marker(
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
    model: &AppModel,
) {
    let focus_similarity_reserved_width = if row.focused && !model.browser.duplicate_cleanup_active
    {
        browser_similarity_button_reserved_width(true, style.sizing)
    } else {
        0.0
    };
    let Some(marker_rect) =
        browser_playback_age_marker_rect(row.rect, style.sizing, focus_similarity_reserved_width)
    else {
        return;
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: marker_rect,
            color: browser_playback_age_marker_color(style, row.playback_age_bucket),
        }),
    );
}

fn emit_locked_marker(
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
    row_border_stroke: f32,
) {
    if !row.locked {
        return;
    }
    let focus_left_border_width = if row.focused { row_border_stroke } else { 0.0 };
    let Some(marker_rect) =
        browser_locked_marker_rect(row.rect, style.sizing, focus_left_border_width)
    else {
        return;
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: marker_rect,
            color: style.accent_mint,
        }),
    );
}
