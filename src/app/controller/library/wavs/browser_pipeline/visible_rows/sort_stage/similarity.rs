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
        controller.settings.similarity.cache_key(),
    ));
    if controller.ui_cache.browser.pipeline.sorted_fingerprint == Some(sorted_fingerprint) {
        return;
    }

    let rating_filter = controller.ui.browser.search.rating_filter.clone();
    let playback_age_filter = controller.ui.browser.search.playback_age_filter.clone();
    let sidebar_filters = controller.ui.browser.search.sidebar_filters.clone();
    let filter = controller.ui.browser.search.filter;
    let marked_only = controller.ui.browser.search.marked_only;
    let tag_named_filter = controller.ui.browser.search.tag_named_filter;
    let selected_source_id = controller.selection_state.ctx.selected_source.clone();
    preload_sidebar_bpm_values(controller, &similar.indices, &sidebar_filters);
    let mut visible = Vec::with_capacity(similar.indices.len());
    for index in similar.indices.iter().copied() {
        let Some((relative_path, tag, locked, last_played_at, marked, tag_named)) =
            filter_stage_entry(
                controller,
                index,
                marked_only.then_some(selected_source_id.as_ref()).flatten(),
            )
        else {
            continue;
        };
        let bpm = sidebar_filters
            .needs_bpm_metadata()
            .then(|| controller.bpm_value_for_path(&relative_path))
            .flatten();
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
            &sidebar_filters,
            &relative_path,
            bpm,
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
) -> Option<(std::path::PathBuf, Rating, bool, Option<i64>, bool, bool)> {
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
        entry.relative_path.clone(),
        entry.tag,
        entry.locked,
        entry.last_played_at,
        marked,
        entry.tag_named,
    ))
}

/// Preload BPM values needed by active sidebar BPM filters before similarity iteration.
fn preload_sidebar_bpm_values(
    controller: &mut AppController,
    candidate_rows: &[usize],
    sidebar_filters: &crate::app::state::BrowserSidebarFilterState,
) {
    if !sidebar_filters.needs_bpm_metadata() {
        return;
    }
    let paths = candidate_rows
        .iter()
        .filter_map(|index| {
            controller
                .ui_cache
                .browser
                .pipeline
                .compact_entries
                .get(*index)
                .map(|entry| entry.relative_path.clone())
        })
        .collect::<Vec<_>>();
    controller.preload_bpm_values_for_paths(&paths);
}
