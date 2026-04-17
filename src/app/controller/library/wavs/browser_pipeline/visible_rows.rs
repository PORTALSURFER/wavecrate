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

fn ensure_filtered_stage(
    controller: &mut AppController,
    filter: TriageFlagFilter,
    rating_filter: &std::collections::BTreeSet<i8>,
    rating_filter_hash: u64,
    playback_age_filter: &std::collections::BTreeSet<crate::app::state::PlaybackAgeFilterChip>,
    playback_age_filter_hash: u64,
    playback_age_cache_token: Option<i64>,
    marked_only: bool,
    playback_age_now_unix_secs: i64,
    marked_revision: u64,
    selected_source_id: Option<&crate::sample_sources::SourceId>,
    folder_hash: u64,
) -> u64 {
    let filtered_fingerprint = filtered_stage_fingerprint(
        controller,
        filter,
        rating_filter_hash,
        playback_age_filter_hash,
        playback_age_cache_token,
        marked_only,
        marked_revision,
        folder_hash,
    );
    if controller.ui_cache.browser.pipeline.filtered_fingerprint != Some(filtered_fingerprint) {
        if let Some(retained_rows) = retained_filter_only_rows(
            controller,
            filter,
            rating_filter,
            playback_age_filter,
            marked_only,
        ) {
            controller.ui_cache.browser.pipeline.filtered_rows = retained_rows.to_vec();
            controller.ui_cache.browser.pipeline.filtered_fingerprint = Some(filtered_fingerprint);
            controller.ui_cache.browser.pipeline.scored_fingerprint = None;
            controller.ui_cache.browser.pipeline.sorted_fingerprint = None;
            return filtered_fingerprint;
        }

        let (candidate_rows, needs_folder_check) = filtered_stage_candidates(controller, filter);
        let mut filtered_rows = Vec::with_capacity(candidate_rows.len());
        for &index in candidate_rows {
            let Some((tag, locked, last_played_at, marked)) = filter_stage_entry(
                controller,
                index,
                marked_only.then_some(selected_source_id).flatten(),
            ) else {
                continue;
            };
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
            if needs_folder_check && !folder_accepts(controller, index) {
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

fn retained_filter_only_rows<'a>(
    controller: &'a AppController,
    filter: TriageFlagFilter,
    rating_filter: &std::collections::BTreeSet<i8>,
    playback_age_filter: &std::collections::BTreeSet<crate::app::state::PlaybackAgeFilterChip>,
    marked_only: bool,
) -> Option<&'a [usize]> {
    if marked_only || !rating_filter.is_empty() || !playback_age_filter.is_empty() {
        return None;
    }
    let pipeline = &controller.ui_cache.browser.pipeline;
    if pipeline.folder_accepts_active {
        return (filter == TriageFlagFilter::All)
            .then_some(pipeline.folder_filtered_rows.as_slice());
    }
    match filter {
        TriageFlagFilter::All => None,
        TriageFlagFilter::Keep => Some(pipeline.keep_rows.as_slice()),
        TriageFlagFilter::Trash => Some(pipeline.trash_rows.as_slice()),
        TriageFlagFilter::Untagged => Some(pipeline.neutral_rows.as_slice()),
    }
}

fn filtered_stage_candidates(
    controller: &AppController,
    filter: TriageFlagFilter,
) -> (&[usize], bool) {
    let pipeline = &controller.ui_cache.browser.pipeline;
    if !pipeline.folder_accepts_active {
        return (triage_candidate_rows(pipeline, filter), false);
    }
    if filter == TriageFlagFilter::All {
        return (pipeline.folder_filtered_rows.as_slice(), false);
    }

    let triage_rows = triage_candidate_rows(pipeline, filter);
    let folder_rows = pipeline.folder_filtered_rows.as_slice();
    if triage_rows.len() <= folder_rows.len() {
        (triage_rows, true)
    } else {
        (folder_rows, false)
    }
}

fn triage_candidate_rows(pipeline: &BrowserPipelineCache, filter: TriageFlagFilter) -> &[usize] {
    match filter {
        TriageFlagFilter::All => pipeline.base_rows.as_slice(),
        TriageFlagFilter::Keep => pipeline.keep_rows.as_slice(),
        TriageFlagFilter::Trash => pipeline.trash_rows.as_slice(),
        TriageFlagFilter::Untagged => pipeline.neutral_rows.as_slice(),
    }
}

fn filtered_stage_fingerprint(
    controller: &AppController,
    filter: TriageFlagFilter,
    rating_filter_hash: u64,
    playback_age_filter_hash: u64,
    playback_age_cache_token: Option<i64>,
    marked_only: bool,
    marked_revision: u64,
    folder_hash: u64,
) -> u64 {
    let base_fingerprint_hash =
        helpers::hash_value(&controller.ui_cache.browser.pipeline.base_fingerprint);
    helpers::hash_value(&(
        base_fingerprint_hash,
        helpers::filter_key(filter),
        rating_filter_hash,
        playback_age_filter_hash,
        playback_age_cache_token,
        marked_only,
        marked_only.then_some(marked_revision),
        folder_hash,
    ))
}

fn ensure_sorted_stage_for_similar(
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

fn current_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
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
    controller
        .ui_cache
        .browser
        .pipeline
        .rebuild_sorted_row_positions(&visible);
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
    controller
        .ui_cache
        .browser
        .pipeline
        .rebuild_sorted_row_positions(&visible);
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
