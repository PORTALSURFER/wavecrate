use super::*;

/// Resolve a sidebar filter-control point to its UI action.
pub(super) fn sidebar_filter_action_at_point(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let rect = sidebar_workspace_sections(layout, style).filters;
    if rect.width() <= 1.0 || rect.height() <= 1.0 || !rect.contains(point) {
        return None;
    }
    let rows = sidebar_filter_row_rects(rect, style.sizing);
    if let Some(facet) = sidebar_filter_dropdown_facet_at_point(&rows, point) {
        shell_state.open_sidebar_filter_dropdown(facet);
        return Some(UiAction::FocusBrowserPanel);
    }
    let Some(rating_row) = rows.get(5).copied() else {
        return None;
    };
    for (index, chip) in sidebar_rating_chip_rects(rating_row, style.sizing)
        .into_iter()
        .enumerate()
    {
        if chip.contains(point) {
            return Some(UiAction::ToggleBrowserRatingFilter {
                level: [-3, -2, -1, 0, 1, 2, 3, 4][index],
                invert: false,
            });
        }
    }
    if rating_row.contains(point) {
        shell_state.open_sidebar_filter_dropdown(SidebarFilterDropdownFacet::Rating);
        return Some(UiAction::FocusBrowserPanel);
    }
    if model.browser.marked_filter_active && rows.first().is_some_and(|row| row.contains(point)) {
        return Some(UiAction::ToggleBrowserMarkedFilter);
    }
    None
}

/// Resolve non-rating sidebar filter rows to their dropdown facets.
fn sidebar_filter_dropdown_facet_at_point(
    rows: &[Rect],
    point: Point,
) -> Option<SidebarFilterDropdownFacet> {
    if rows.first().is_some_and(|row| row.contains(point)) {
        Some(SidebarFilterDropdownFacet::Format)
    } else if rows.get(1).is_some_and(|row| row.contains(point)) {
        Some(SidebarFilterDropdownFacet::BitDepth)
    } else if rows.get(2).is_some_and(|row| row.contains(point)) {
        Some(SidebarFilterDropdownFacet::Channels)
    } else if rows.get(3).is_some_and(|row| row.contains(point)) {
        Some(SidebarFilterDropdownFacet::Bpm)
    } else if rows.get(4).is_some_and(|row| row.contains(point)) {
        Some(SidebarFilterDropdownFacet::Key)
    } else {
        None
    }
}

/// Return local sidebar filter row rectangles.
pub(super) fn sidebar_filter_row_rects(rect: Rect, sizing: SizingTokens) -> Vec<Rect> {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 2.0;
    let title_height = sizing.font_meta + sizing.text_inset_y + 4.0;
    let available = (rect.height() - pad * 2.0 - title_height - gap * 5.0).max(0.0);
    let row_height = (available / 6.0)
        .min(sizing.browser_row_height.max(18.0))
        .max(8.0);
    (0..6)
        .map(|index| {
            let min_y = rect.min.y + pad + title_height + (row_height + gap) * index as f32;
            Rect::from_min_max(
                Point::new(rect.min.x + pad, min_y),
                Point::new(rect.max.x - pad, (min_y + row_height).min(rect.max.y - pad)),
            )
        })
        .collect()
}

/// Return local sidebar rating-chip hit rectangles.
pub(super) fn sidebar_rating_chip_rects(rating_row: Rect, sizing: SizingTokens) -> [Rect; 8] {
    let chip_gap = 2.0_f32.max(sizing.border_width);
    let left = rating_row.min.x + (rating_row.width() * 0.43);
    let right = rating_row.max.x - sizing.text_inset_x;
    let available = (right - left - chip_gap * 7.0).max(0.0);
    let side = (available / 8.0).min(rating_row.height() - 4.0).max(0.0);
    std::array::from_fn(|index| {
        let x = left + (side + chip_gap) * index as f32;
        Rect::from_min_max(
            Point::new(x, rating_row.min.y + 2.0),
            Point::new((x + side).min(right), rating_row.min.y + 2.0 + side),
        )
    })
}
