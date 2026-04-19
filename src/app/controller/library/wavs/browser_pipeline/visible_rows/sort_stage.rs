use super::super::folder_stage::folder_accepts;
use super::*;

use crate::app::state::SampleBrowserSort;

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
    let selected_source_id = controller.selection_state.ctx.selected_source.clone();
    let mut visible = Vec::with_capacity(similar.indices.len());
    for index in similar.indices.iter().copied() {
        let Some((tag, locked, last_played_at, marked)) = filter_stage_entry(
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

pub(super) fn ensure_sorted_stage_for_query(
    controller: &mut AppController,
    filtered_fingerprint: u64,
    sort_mode: SampleBrowserSort,
    query: &str,
) {
    controller.ensure_search_scores(query);
    let score_fingerprint = helpers::hash_value(&(
        filtered_fingerprint,
        helpers::hash_value(query),
        controller.ui_cache.browser.search.scores.len(),
    ));
    if controller.ui_cache.browser.pipeline.scored_fingerprint != Some(score_fingerprint) {
        let filtered_len = controller.ui_cache.browser.pipeline.filtered_rows.len();
        let mut scored = Vec::with_capacity(filtered_len);
        for row in 0..filtered_len {
            let index = controller.ui_cache.browser.pipeline.filtered_rows[row];
            if let Some(score) = controller
                .ui_cache
                .browser
                .search
                .scores
                .get(index)
                .and_then(|score| *score)
            {
                scored.push((index, score));
            }
        }
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        controller.ui_cache.browser.pipeline.scored_rows = scored;
        controller.ui_cache.browser.pipeline.scored_fingerprint = Some(score_fingerprint);
        controller.ui_cache.browser.pipeline.sorted_fingerprint = None;
    }

    let sorted_fingerprint = helpers::hash_value(&(
        controller.ui_cache.browser.pipeline.scored_fingerprint,
        helpers::sort_key(sort_mode),
    ));
    if controller.ui_cache.browser.pipeline.sorted_fingerprint == Some(sorted_fingerprint) {
        return;
    }

    let mut visible: Vec<usize> = controller
        .ui_cache
        .browser
        .pipeline
        .scored_rows
        .iter()
        .map(|(index, _)| *index)
        .collect();
    maybe_sort_visible_by_playback_age(controller, &mut visible, sort_mode);
    controller
        .ui_cache
        .browser
        .pipeline
        .rebuild_sorted_row_positions(&visible);
    controller.ui_cache.browser.pipeline.sorted_rows = visible.into();
    controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
}

pub(super) fn ensure_sorted_stage_for_filter_only(
    controller: &mut AppController,
    filtered_fingerprint: u64,
    sort_mode: SampleBrowserSort,
) {
    let sorted_fingerprint =
        helpers::hash_value(&(filtered_fingerprint, helpers::sort_key(sort_mode)));
    if controller.ui_cache.browser.pipeline.sorted_fingerprint == Some(sorted_fingerprint) {
        return;
    }

    let mut visible = controller.ui_cache.browser.pipeline.filtered_rows.clone();
    maybe_sort_visible_by_playback_age(controller, &mut visible, sort_mode);
    controller
        .ui_cache
        .browser
        .pipeline
        .rebuild_sorted_row_positions(&visible);
    controller.ui_cache.browser.pipeline.sorted_rows = visible.into();
    controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
}

fn maybe_sort_visible_by_playback_age(
    controller: &mut AppController,
    visible: &mut Vec<usize>,
    sort_mode: SampleBrowserSort,
) {
    if matches!(
        sort_mode,
        SampleBrowserSort::PlaybackAgeAsc | SampleBrowserSort::PlaybackAgeDesc
    ) {
        helpers::sort_visible_by_playback_age(
            controller,
            visible,
            sort_mode == SampleBrowserSort::PlaybackAgeAsc,
        );
    }
}

fn filter_stage_entry(
    controller: &AppController,
    index: usize,
    selected_source_id: Option<&crate::sample_sources::SourceId>,
) -> Option<(Rating, bool, Option<i64>, bool)> {
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
    Some((entry.tag, entry.locked, entry.last_played_at, marked))
}
