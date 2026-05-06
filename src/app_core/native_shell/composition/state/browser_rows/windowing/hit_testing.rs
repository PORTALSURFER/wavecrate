use super::*;
use crate::gui::list::{
    MaterializedVirtualListItem, VirtualListItemKey, virtual_list_stacked_item_at_point,
};

pub(in crate::app_core::native_shell::composition::state) fn row_index_for_visible_rows(
    rows: &[CachedBrowserRow],
    point: Point,
    browser_rows: Rect,
) -> Option<usize> {
    if rows.is_empty() || !browser_rows.contains(point) {
        return None;
    }
    row_index_for_stacked_geometry(rows, point)
}

/// Resolve one browser-row index from stacked row geometry in constant time.
pub(in crate::app_core::native_shell::composition::state) fn row_index_for_stacked_geometry(
    rows: &[CachedBrowserRow],
    point: Point,
) -> Option<usize> {
    let items = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            MaterializedVirtualListItem::new(
                VirtualListItemKey(row.visible_row as u64),
                index,
                row.rect,
            )
        })
        .collect::<Vec<_>>();
    virtual_list_stacked_item_at_point(&items, point)
}
