use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::{Point, Rect};

/// Build the top header band for one shell panel.
pub(super) fn band_header(panel: Rect, header_height: f32) -> Rect {
    Rect::from_min_max(
        panel.min,
        Point::new(panel.max.x, (panel.min.y + header_height).min(panel.max.y)),
    )
}

/// Build the legacy invisible browser triage columns used by compatibility hit-testing.
pub(super) fn build_browser_compat_columns(browser_rows: Rect, sizing: SizingTokens) -> [Rect; 3] {
    let base_column_width =
        ((browser_rows.width() - (sizing.column_gap * 2.0)) / 3.0).max(sizing.column_min_width);
    let mut columns = [Rect::default(), Rect::default(), Rect::default()];
    for (index, column) in columns.iter_mut().enumerate() {
        let x0 = browser_rows.min.x + (base_column_width + sizing.column_gap) * index as f32;
        let x1 = if index == 2 {
            browser_rows.max.x
        } else {
            x0 + base_column_width
        };
        *column = Rect::from_min_max(
            Point::new(x0, browser_rows.min.y),
            Point::new(x1, browser_rows.max.y),
        );
    }
    columns
}

/// Build per-column header and row sections for the compatibility geometry.
pub(super) fn build_column_sections(
    columns: [Rect; 3],
    sizing: SizingTokens,
) -> ([Rect; 3], [Rect; 3]) {
    let mut column_headers = [Rect::default(), Rect::default(), Rect::default()];
    let mut column_rows = [Rect::default(), Rect::default(), Rect::default()];
    for (index, column) in columns.iter().copied().enumerate() {
        let header = band_header(column, sizing.column_header_block_height);
        let rows_top = (header.max.y + sizing.header_to_rows_gap).min(column.max.y);
        let rows_bottom = (column.max.y - sizing.column_bottom_padding).max(header.max.y);
        column_headers[index] = header;
        column_rows[index] = Rect::from_min_max(
            Point::new(column.min.x, rows_top),
            Point::new(column.max.x, rows_bottom),
        )
        .inset_horizontal_saturating(sizing.panel_inset);
    }
    (column_headers, column_rows)
}

/// Compute the reserved lane height for the waveform scrollbar track.
pub(super) fn waveform_scrollbar_lane_height(waveform_body: Rect, header_height: f32) -> f32 {
    if waveform_body.height() <= 1.0 {
        return 0.0;
    }
    let desired = (header_height * 0.5).round().clamp(12.0, 18.0);
    desired.min((waveform_body.height() - 1.0).max(0.0))
}
