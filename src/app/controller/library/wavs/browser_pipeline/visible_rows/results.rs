use super::*;
use crate::app::state::BrowserDuplicateCleanupState;
use crate::app::state::VisibleRows;
use std::sync::Arc;

/// Project duplicate-cleanup indices into visible rows and remap focus/loading.
pub(super) fn visible_result_for_duplicate_cleanup(
    controller: &mut AppController,
    focused_index: Option<usize>,
    loaded_index: Option<usize>,
    cleanup: &BrowserDuplicateCleanupState,
) -> (VisibleRows, Option<usize>, Option<usize>) {
    let entries_len = controller.ui_cache.browser.pipeline.compact_entries.len();
    let mut selected_visible = None;
    let mut loaded_visible = None;
    let visible: Arc<[usize]> = cleanup
        .indices
        .iter()
        .copied()
        .filter(|index| *index < entries_len)
        .enumerate()
        .map(|(visible_row, index)| {
            if focused_index == Some(index) {
                selected_visible = Some(visible_row);
            }
            if loaded_index == Some(index) {
                loaded_visible = Some(visible_row);
            }
            index
        })
        .collect::<Vec<_>>()
        .into();
    (VisibleRows::List(visible), selected_visible, loaded_visible)
}

/// Return the visible-row output already retained in the sorted-stage cache.
pub(super) fn visible_result_from_sorted_stage(
    controller: &mut AppController,
    focused_index: Option<usize>,
    loaded_index: Option<usize>,
) -> (VisibleRows, Option<usize>, Option<usize>) {
    let visible = Arc::clone(&controller.ui_cache.browser.pipeline.sorted_rows);
    let selected_visible = focused_index.and_then(|index| {
        controller
            .ui_cache
            .browser
            .pipeline
            .sorted_visible_position(index)
    });
    let loaded_visible = loaded_index.and_then(|index| {
        controller
            .ui_cache
            .browser
            .pipeline
            .sorted_visible_position(index)
    });
    (VisibleRows::List(visible), selected_visible, loaded_visible)
}
