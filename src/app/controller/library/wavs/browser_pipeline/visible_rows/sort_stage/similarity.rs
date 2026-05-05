use super::super::super::folder_stage::folder_accepts;
use super::super::*;

use crate::app::state::SampleBrowserSort;

/// Rebuild visible rows for the active similarity query after filter checks.
pub(super) fn ensure_sorted_stage_for_similar(
    controller: &mut AppController,
    filtered_fingerprint: u64,
    sort_mode: SampleBrowserSort,
    similar: &crate::app::state::SimilarQuery,
    playback_age_now_unix_secs: i64,
) {
    let sorted_fingerprint = helpers::hash_value(&(
        filtered_fingerprint,
        helpers::sort_key(sort_mode),
        helpers::similarity_fingerprint(similar),
    ));
    if controller.ui_cache.browser.pipeline.sorted_fingerprint == Some(sorted_fingerprint) {
        return;
    }

    let rating_filter = controller.ui.browser.search.rating_filter.clone();
    let playback_age_filter = controller.ui.browser.search.playback_age_filter.clone();
    let filter = controller.ui.browser.search.filter;
    let marked_only = controller.ui.browser.search.marked_only;
    let tag_named_filter = controller.ui.browser.search.tag_named_filter;
    let selected_source_id = controller.selection_state.ctx.selected_source.clone();
    let mut visible = Vec::with_capacity(similar.indices.len());
    for index in similar.indices.iter().copied() {
        let Some((tag, locked, last_played_at, marked, tag_named)) = filter_stage_entry(
            controller,
            index,
            marked_only.then_some(selected_source_id.as_ref()).flatten(),
        ) else {
            continue;
        };
        if !helpers::filter_accepts(
            filter,
            &rating_filter,
            &playback_age_filter,
            marked_only,
            marked,
            tag_named_filter,
            tag_named,
            tag,
            locked,
            last_played_at,
            playback_age_now_unix_secs,
        ) {
            continue;
        }
        if !folder_accepts(controller, index) {
            continue;
        }
        visible.push(index);
    }
    helpers::apply_sort_for_similar(controller, &mut visible, sort_mode, similar);
    controller
        .ui_cache
        .browser
        .pipeline
        .rebuild_sorted_row_positions(&visible);
    controller.ui_cache.browser.pipeline.sorted_rows = visible.into();
    controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
}

fn filter_stage_entry(
    controller: &AppController,
    index: usize,
    selected_source_id: Option<&crate::sample_sources::SourceId>,
) -> Option<(Rating, bool, Option<i64>, bool, bool)> {
    let entry = controller
        .ui_cache
        .browser
        .pipeline
        .compact_entries
        .get(index)?;
    let marked = selected_source_id.is_some_and(|source_id| {
        controller
            .ui
            .browser
            .marks
            .contains(source_id, &entry.relative_path)
    });
    Some((
        entry.tag,
        entry.locked,
        entry.last_played_at,
        marked,
        entry.tag_named,
    ))
}
