use super::base_stage::ensure_base_stage;
use super::folder_stage::ensure_folder_acceptance_stage;
use super::*;
use crate::app::state::BrowserDuplicateCleanupState;
use crate::app::state::{SampleBrowserSort, TriageFlagFilter, VisibleRows};
use std::time::{SystemTime, UNIX_EPOCH};

mod filter_stage;
mod sort_stage;

use self::filter_stage::{ensure_filtered_stage, filtered_stage_fingerprint};
use self::sort_stage::{
    ensure_sorted_stage_for_filter_only, ensure_sorted_stage_for_query,
    ensure_sorted_stage_for_similar,
};

/// Build browser visible rows from retained staged pipeline caches.
pub(crate) fn build_visible_rows(
    controller: &mut AppController,
    focused_index: Option<usize>,
    loaded_index: Option<usize>,
) -> (VisibleRows, Option<usize>, Option<usize>) {
    build_visible_rows_with_now(controller, focused_index, loaded_index, current_unix_secs())
}

/// Build browser visible rows using one explicit playback-age timestamp.
pub(super) fn build_visible_rows_with_now(
    controller: &mut AppController,
    focused_index: Option<usize>,
    loaded_index: Option<usize>,
    playback_age_now_unix_secs: i64,
) -> (VisibleRows, Option<usize>, Option<usize>) {
    ensure_base_stage(controller);

    let duplicate_cleanup = controller.ui.browser.duplicate_cleanup.clone();
    let query = controller.active_search_query().map(str::to_owned);
    let similar_query = controller.ui.browser.search.similar_query.clone();
    let sort_mode = controller.ui.browser.search.sort;
    let filter = controller.ui.browser.search.filter;
    let rating_filter = controller.ui.browser.search.rating_filter.clone();
    let rating_filter_hash = helpers::hash_value(&rating_filter);
    let playback_age_filter = controller.ui.browser.search.playback_age_filter.clone();
    let playback_age_filter_hash = helpers::hash_value(&playback_age_filter);
    let playback_age_cache_token = helpers::playback_age_filter_cache_token(
        controller,
        &playback_age_filter,
        playback_age_now_unix_secs,
    );
    let marked_only = controller.ui.browser.search.marked_only;
    let marked_revision = controller.ui.browser.marks.revision;
    let selected_source_id = controller.selection_state.ctx.selected_source.clone();
    let folder_selection = controller.folder_selection_for_filter().cloned();
    let folder_negated = controller.folder_negation_for_filter().cloned();
    let file_scope_mode = controller
        .folder_file_scope_mode_for_filter()
        .unwrap_or_default();
    let folder_hash = crate::app::controller::library::source_folders::folder_filter_fingerprint(
        folder_selection.as_ref(),
        folder_negated.as_ref(),
        file_scope_mode,
    );
    let has_folder_filters = crate::app::controller::library::source_folders::folder_filters_active(
        folder_selection.as_ref(),
        folder_negated.as_ref(),
        file_scope_mode,
    );
    ensure_folder_acceptance_stage(
        controller,
        folder_selection.as_ref(),
        folder_negated.as_ref(),
        file_scope_mode,
        folder_hash,
        has_folder_filters,
    );

    if let Some(cleanup) = duplicate_cleanup {
        return visible_result_for_duplicate_cleanup(
            controller,
            focused_index,
            loaded_index,
            &cleanup,
        );
    }

    if query.is_none()
        && similar_query.is_none()
        && sort_mode == SampleBrowserSort::ListOrder
        && filter == TriageFlagFilter::All
        && controller.ui.browser.search.rating_filter.is_empty()
        && controller.ui.browser.search.playback_age_filter.is_empty()
        && !marked_only
        && !has_folder_filters
    {
        let total = controller.ui_cache.browser.pipeline.compact_entries.len();
        return (VisibleRows::All { total }, focused_index, loaded_index);
    }

    let filter_fingerprint = filtered_stage_fingerprint(
        controller,
        filter,
        rating_filter_hash,
        playback_age_filter_hash,
        playback_age_cache_token,
        marked_only,
        marked_revision,
        folder_hash,
    );

    if let Some(similar) = similar_query {
        ensure_sorted_stage_for_similar(
            controller,
            filter_fingerprint,
            sort_mode,
            &similar,
            playback_age_now_unix_secs,
        );
        return visible_result_from_sorted(controller, focused_index, loaded_index);
    }

    let filtered_fingerprint = ensure_filtered_stage(
        controller,
        filter,
        &rating_filter,
        rating_filter_hash,
        &playback_age_filter,
        playback_age_filter_hash,
        playback_age_cache_token,
        marked_only,
        playback_age_now_unix_secs,
        marked_revision,
        selected_source_id.as_ref(),
        folder_hash,
    );

    if let Some(query) = query {
        ensure_sorted_stage_for_query(controller, filtered_fingerprint, sort_mode, &query);
        return visible_result_from_sorted(controller, focused_index, loaded_index);
    }

    ensure_sorted_stage_for_filter_only(controller, filtered_fingerprint, sort_mode);
    visible_result_from_sorted(controller, focused_index, loaded_index)
}

fn visible_result_for_duplicate_cleanup(
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

/// Return the visible rows output from the sorted stage cache.
fn visible_result_from_sorted(
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

fn current_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
