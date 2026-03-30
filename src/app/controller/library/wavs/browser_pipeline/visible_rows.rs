use super::base_stage::ensure_base_stage;
use super::folder_stage::{ensure_folder_acceptance_stage, folder_accepts};
use super::*;
use crate::app::state::BrowserDuplicateCleanupState;
use crate::app::state::{SampleBrowserSort, TriageFlagFilter, VisibleRows};
use std::time::{SystemTime, UNIX_EPOCH};

/// Build browser visible rows from retained staged pipeline caches.
pub(crate) fn build_visible_rows(
    controller: &mut AppController,
    focused_index: Option<usize>,
    loaded_index: Option<usize>,
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
    let marked_only = controller.ui.browser.search.marked_only;
    let playback_age_now_unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
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
        return visible_result_for_duplicate_cleanup(controller, focused_index, loaded_index, &cleanup);
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
        let total = controller.wav_entries_len();
        return (VisibleRows::All { total }, focused_index, loaded_index);
    }

    let filtered_fingerprint = ensure_filtered_stage(
        controller,
        filter,
        &rating_filter,
        rating_filter_hash,
        &playback_age_filter,
        playback_age_filter_hash,
        marked_only,
        playback_age_now_unix_secs,
        marked_revision,
        selected_source_id.as_ref(),
        folder_hash,
    );

    if let Some(similar) = similar_query {
        ensure_sorted_stage_for_similar(controller, filtered_fingerprint, sort_mode, &similar);
        return visible_result_from_sorted(controller, focused_index, loaded_index);
    }

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
    let visible: Arc<[usize]> = cleanup
        .indices
        .iter()
        .copied()
        .filter(|index| controller.wav_entry(*index).is_some())
        .collect::<Vec<_>>()
        .into();
    let selected_visible =
        focused_index.and_then(|index| visible.iter().position(|row| *row == index));
    let loaded_visible =
        loaded_index.and_then(|index| visible.iter().position(|row| *row == index));
    (VisibleRows::List(visible), selected_visible, loaded_visible)
}

fn ensure_filtered_stage(
    controller: &mut AppController,
    filter: TriageFlagFilter,
    rating_filter: &std::collections::BTreeSet<i8>,
    rating_filter_hash: u64,
    playback_age_filter: &std::collections::BTreeSet<crate::app::state::PlaybackAgeFilterChip>,
    playback_age_filter_hash: u64,
    marked_only: bool,
    playback_age_now_unix_secs: i64,
    marked_revision: u64,
    selected_source_id: Option<&crate::sample_sources::SourceId>,
    folder_hash: u64,
) -> u64 {
    let base_fingerprint_hash =
        helpers::hash_value(&controller.ui_cache.browser.pipeline.base_fingerprint);
    let filtered_fingerprint = helpers::hash_value(&(
        base_fingerprint_hash,
        helpers::filter_key(filter),
        rating_filter_hash,
        playback_age_filter_hash,
        (!playback_age_filter.is_empty()).then_some(playback_age_now_unix_secs),
        marked_only,
        marked_revision,
        folder_hash,
    ));
    if controller.ui_cache.browser.pipeline.filtered_fingerprint != Some(filtered_fingerprint) {
        let base_len = controller.ui_cache.browser.pipeline.base_rows.len();
        let mut filtered_rows = Vec::with_capacity(base_len);
        for row in 0..base_len {
            let index = controller.ui_cache.browser.pipeline.base_rows[row];
            let Some((tag, locked, last_played_at, relative_path)) = controller
                .wav_entry(index)
                .map(|entry| {
                    (
                        entry.tag,
                        entry.locked,
                        entry.last_played_at,
                        entry.relative_path.clone(),
                    )
                })
            else {
                continue;
            };
            let marked = selected_source_id
                .is_some_and(|source_id| controller.browser_sample_marked(source_id, &relative_path));
            if !helpers::filter_accepts(
                filter,
                rating_filter,
                playback_age_filter,
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
            filtered_rows.push(index);
        }
        controller.ui_cache.browser.pipeline.filtered_rows = filtered_rows;
        controller.ui_cache.browser.pipeline.filtered_fingerprint = Some(filtered_fingerprint);
        controller.ui_cache.browser.pipeline.scored_fingerprint = None;
        controller.ui_cache.browser.pipeline.sorted_fingerprint = None;
    }
    filtered_fingerprint
}

fn ensure_sorted_stage_for_similar(
    controller: &mut AppController,
    filtered_fingerprint: u64,
    sort_mode: SampleBrowserSort,
    similar: &crate::app::state::SimilarQuery,
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
    let playback_age_now_unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let selected_source_id = controller.selection_state.ctx.selected_source.clone();
    let mut visible = Vec::with_capacity(similar.indices.len());
    for index in similar.indices.iter().copied() {
        let Some((tag, locked, last_played_at, relative_path)) = controller
            .wav_entry(index)
            .map(|entry| {
                (
                    entry.tag,
                    entry.locked,
                    entry.last_played_at,
                    entry.relative_path.clone(),
                )
            })
        else {
            continue;
        };
        let marked = selected_source_id
            .as_ref()
            .is_some_and(|source_id| controller.browser_sample_marked(source_id, &relative_path));
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
    controller.ui_cache.browser.pipeline.sorted_rows = visible.into();
    controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
}

fn ensure_sorted_stage_for_query(
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
    if matches!(
        sort_mode,
        SampleBrowserSort::PlaybackAgeAsc | SampleBrowserSort::PlaybackAgeDesc
    ) {
        helpers::sort_visible_by_playback_age(
            controller,
            &mut visible,
            sort_mode == SampleBrowserSort::PlaybackAgeAsc,
        );
    }
    controller.ui_cache.browser.pipeline.sorted_rows = visible.into();
    controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
}

fn ensure_sorted_stage_for_filter_only(
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
    if matches!(
        sort_mode,
        SampleBrowserSort::PlaybackAgeAsc | SampleBrowserSort::PlaybackAgeDesc
    ) {
        helpers::sort_visible_by_playback_age(
            controller,
            &mut visible,
            sort_mode == SampleBrowserSort::PlaybackAgeAsc,
        );
    }
    controller.ui_cache.browser.pipeline.sorted_rows = visible.into();
    controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
}

/// Return the visible rows output from the sorted stage cache.
fn visible_result_from_sorted(
    controller: &mut AppController,
    focused_index: Option<usize>,
    loaded_index: Option<usize>,
) -> (VisibleRows, Option<usize>, Option<usize>) {
    let visible = Arc::clone(&controller.ui_cache.browser.pipeline.sorted_rows);
    let selected_visible =
        focused_index.and_then(|index| visible.iter().position(|row| *row == index));
    let loaded_visible =
        loaded_index.and_then(|index| visible.iter().position(|row| *row == index));
    (VisibleRows::List(visible), selected_visible, loaded_visible)
}
