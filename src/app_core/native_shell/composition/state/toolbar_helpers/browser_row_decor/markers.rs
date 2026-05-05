use super::*;

/// Return the stroke width used for browser row borders at the current UI scale.
///
/// At `ui_scale == 1.0` this resolves to `1.0` logical px so row borders stay
/// visually consistent at 100% scale.
pub(in crate::gui::native_shell::state) fn browser_row_border_stroke(layout: &ShellLayout) -> f32 {
    layout.ui_scale.max(1.0)
}

/// Return x-advance reserved for the missing-file marker before an item label.
pub(in crate::gui::native_shell::state) fn browser_missing_marker_advance(font_size: f32) -> f32 {
    (font_size * 1.05).max(7.0)
}

/// Return the inset left-edge marker rect used to flag locked browser rows.
///
/// The marker stays inside the row gutter before the numbering column. When a
/// focused row also renders a left focus border, `focused_left_border_width`
/// shifts the marker to the right so both accents remain visible.
pub(in crate::gui::native_shell::state) fn browser_locked_marker_rect(
    row_rect: Rect,
    sizing: SizingTokens,
    focused_left_border_width: f32,
) -> Option<Rect> {
    if row_rect.width() <= 0.0 || row_rect.height() <= 0.0 {
        return None;
    }
    let inset = sizing.row_corner_inset.max(1.0);
    let marker_width = (row_rect.height() * 0.22).clamp(4.0, 6.0);
    let min_x = row_rect.min.x + inset + focused_left_border_width.max(0.0);
    let max_x = (min_x + marker_width).min(row_rect.max.x - inset);
    let min_y = row_rect.min.y + inset;
    let max_y = row_rect.max.y - inset;
    if max_x <= min_x || max_y <= min_y {
        return None;
    }
    Some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(max_x, max_y),
    ))
}

/// Return the compact sample-column marker rect used to show playback age.
///
/// The marker sits at the leading edge of the sample column. Focused rows can
/// reserve extra leading width for the similarity button so both controls stay
/// visible without overlapping.
pub(in crate::gui::native_shell::state) fn browser_playback_age_marker_rect(
    row_rect: Rect,
    sizing: SizingTokens,
    leading_reserved_width: f32,
) -> Option<Rect> {
    if row_rect.width() <= 0.0 || row_rect.height() <= 0.0 {
        return None;
    }
    let sample_column = compute_browser_row_text_layout(row_rect, sizing)
        .columns
        .sample;
    if sample_column.width() <= 0.0 || sample_column.height() <= 0.0 {
        return None;
    }
    let inset_x = sizing.text_inset_x.clamp(2.0, 6.0);
    let inset_y = sizing.row_corner_inset.max(2.0);
    let width = (row_rect.height() * 0.22).clamp(6.0, 9.0);
    let min_x = sample_column.min.x + inset_x + leading_reserved_width.max(0.0);
    let max_x = (min_x + width).min(sample_column.max.x - inset_x);
    let min_y = row_rect.min.y + inset_y;
    let max_y = row_rect.max.y - inset_y;
    if max_x <= min_x || max_y <= min_y {
        return None;
    }
    Some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(max_x, max_y),
    ))
}

/// Return the horizontal width reserved for the playback-age marker and gap.
pub(in crate::gui::native_shell::state) fn browser_playback_age_marker_reserved_width(
    row_rect: Rect,
    sizing: SizingTokens,
    leading_reserved_width: f32,
) -> f32 {
    browser_playback_age_marker_rect(row_rect, sizing, leading_reserved_width)
        .map(|rect| rect.width() + sizing.text_inset_x.clamp(4.0, 8.0))
        .unwrap_or(0.0)
}

/// Snap browser-row border bounds to the border stroke grid to avoid uneven AA
/// widths between top/bottom edges.
pub(in crate::gui::native_shell::state) fn browser_row_border_rect(
    rect: Rect,
    stroke: f32,
) -> Rect {
    let stroke = stroke.max(1.0);
    let snap = |value: f32| (value / stroke).round() * stroke;
    let min_x = snap(rect.min.x);
    let min_y = snap(rect.min.y);
    let max_x = snap(rect.max.x);
    let max_y = snap(rect.max.y);
    let snapped = Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y));
    if snapped.width() <= stroke * 2.0 || snapped.height() <= stroke * 2.0 {
        rect
    } else {
        snapped
    }
}
