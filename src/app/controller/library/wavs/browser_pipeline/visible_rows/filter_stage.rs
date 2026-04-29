use super::super::folder_stage::folder_accepts;
use super::*;

pub(super) fn ensure_filtered_stage(
    controller: &mut AppController,
    filter: TriageFlagFilter,
    rating_filter: &std::collections::BTreeSet<i8>,
    rating_filter_hash: u64,
    playback_age_filter: &std::collections::BTreeSet<crate::app::state::PlaybackAgeFilterChip>,
    playback_age_filter_hash: u64,
    playback_age_cache_token: Option<i64>,
    marked_only: bool,
    tag_named_filter: crate::app::state::TagNamedFilter,
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
        tag_named_filter,
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
            tag_named_filter,
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
            let Some((tag, locked, last_played_at, marked, tag_named)) = filter_stage_entry(
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
                tag_named_filter,
                tag_named,
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

pub(super) fn filtered_stage_fingerprint(
    controller: &AppController,
    filter: TriageFlagFilter,
    rating_filter_hash: u64,
    playback_age_filter_hash: u64,
    playback_age_cache_token: Option<i64>,
    marked_only: bool,
    tag_named_filter: crate::app::state::TagNamedFilter,
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
        tag_named_filter,
        marked_only.then_some(marked_revision),
        folder_hash,
    ))
}

fn retained_filter_only_rows<'a>(
    controller: &'a AppController,
    filter: TriageFlagFilter,
    rating_filter: &std::collections::BTreeSet<i8>,
    playback_age_filter: &std::collections::BTreeSet<crate::app::state::PlaybackAgeFilterChip>,
    marked_only: bool,
    tag_named_filter: crate::app::state::TagNamedFilter,
) -> Option<&'a [usize]> {
    if marked_only
        || !rating_filter.is_empty()
        || !playback_age_filter.is_empty()
        || tag_named_filter != crate::app::state::TagNamedFilter::All
    {
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
