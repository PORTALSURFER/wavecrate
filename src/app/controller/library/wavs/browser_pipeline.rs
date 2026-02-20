use super::*;
use crate::app::state::{SampleBrowserSort, TriageFlagFilter, VisibleRows};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// Cache state for retained browser pipeline stages.
#[derive(Default)]
pub(crate) struct BrowserPipelineCache {
    /// Fingerprint for the current base row snapshot.
    base_fingerprint: Option<BaseStageFingerprint>,
    /// Absolute entry indices in source list order.
    base_rows: Vec<usize>,
    /// Cached triage trash bucket in source list order.
    pub(crate) trash_rows: Vec<usize>,
    /// Cached triage neutral bucket in source list order.
    pub(crate) neutral_rows: Vec<usize>,
    /// Cached triage keep bucket in source list order.
    pub(crate) keep_rows: Vec<usize>,
    /// Fingerprint for the filtered stage rows.
    filtered_fingerprint: Option<u64>,
    /// Filtered absolute entry indices.
    filtered_rows: Vec<usize>,
    /// Fingerprint for the scored stage rows.
    scored_fingerprint: Option<u64>,
    /// Scored rows in descending fuzzy-score order.
    scored_rows: Vec<(usize, i64)>,
    /// Fingerprint for the sorted stage rows.
    sorted_fingerprint: Option<u64>,
    /// Sorted visible absolute entry indices.
    sorted_rows: Vec<usize>,
}

impl BrowserPipelineCache {
    /// Drop all staged fingerprints and vectors.
    pub(crate) fn invalidate(&mut self) {
        self.base_fingerprint = None;
        self.base_rows.clear();
        self.trash_rows.clear();
        self.neutral_rows.clear();
        self.keep_rows.clear();
        self.filtered_fingerprint = None;
        self.filtered_rows.clear();
        self.scored_fingerprint = None;
        self.scored_rows.clear();
        self.sorted_fingerprint = None;
        self.sorted_rows.clear();
    }
}

/// Stable identity for the stage-A base snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct BaseStageFingerprint {
    source_id: Option<SourceId>,
    source_revision: Option<u64>,
    entries_len: usize,
}

/// Build browser visible rows from retained staged pipeline caches.
pub(crate) fn build_visible_rows(
    controller: &mut AppController,
    focused_index: Option<usize>,
    loaded_index: Option<usize>,
) -> (VisibleRows, Option<usize>, Option<usize>) {
    ensure_base_stage(controller);

    let query = controller.active_search_query().map(str::to_owned);
    let similar_query = controller.ui.browser.similar_query.clone();
    let sort_mode = controller.ui.browser.sort;
    let filter = controller.ui.browser.filter;
    let rating_filter = controller.ui.browser.rating_filter.clone();
    let rating_filter_hash = hash_value(&rating_filter);
    let folder_selection = controller.folder_selection_for_filter().cloned();
    let folder_negated = controller.folder_negation_for_filter().cloned();
    let root_mode = controller
        .root_folder_filter_mode_for_filter()
        .unwrap_or_default();
    let folder_hash = hash_value(&(
        folder_selection.as_ref(),
        folder_negated.as_ref(),
        root_mode_key(root_mode),
    ));
    let has_folder_filters = crate::app::controller::library::source_folders::folder_filters_active(
        folder_selection.as_ref(),
        folder_negated.as_ref(),
        root_mode,
    );

    if query.is_none()
        && similar_query.is_none()
        && sort_mode == SampleBrowserSort::ListOrder
        && filter == TriageFlagFilter::All
        && controller.ui.browser.rating_filter.is_empty()
        && !has_folder_filters
    {
        let total = controller.wav_entries_len();
        return (VisibleRows::All { total }, focused_index, loaded_index);
    }

    let base_fingerprint_hash = hash_value(&controller.ui_cache.browser.pipeline.base_fingerprint);
    let filtered_fingerprint = hash_value(&(
        base_fingerprint_hash,
        filter_key(filter),
        rating_filter_hash,
        folder_hash,
    ));
    if controller.ui_cache.browser.pipeline.filtered_fingerprint != Some(filtered_fingerprint) {
        let base_rows = controller.ui_cache.browser.pipeline.base_rows.clone();
        let mut filtered_rows = Vec::with_capacity(base_rows.len());
        for index in base_rows {
            let Some(entry) = controller.wav_entry(index) else {
                continue;
            };
            if !filter_accepts(filter, &rating_filter, entry.tag) {
                continue;
            }
            if !crate::app::controller::library::source_folders::folder_filter_accepts(
                &entry.relative_path,
                folder_selection.as_ref(),
                folder_negated.as_ref(),
                root_mode,
            ) {
                continue;
            }
            filtered_rows.push(index);
        }
        controller.ui_cache.browser.pipeline.filtered_rows = filtered_rows;
        controller.ui_cache.browser.pipeline.filtered_fingerprint = Some(filtered_fingerprint);
        controller.ui_cache.browser.pipeline.scored_fingerprint = None;
        controller.ui_cache.browser.pipeline.sorted_fingerprint = None;
    }

    if let Some(similar) = similar_query {
        let sorted_fingerprint = hash_value(&(
            filtered_fingerprint,
            sort_key(sort_mode),
            similarity_fingerprint(&similar),
        ));
        if controller.ui_cache.browser.pipeline.sorted_fingerprint != Some(sorted_fingerprint) {
            let mut visible = Vec::with_capacity(similar.indices.len());
            for index in similar.indices.iter().copied() {
                let Some(entry) = controller.wav_entry(index) else {
                    continue;
                };
                if !filter_accepts(filter, &rating_filter, entry.tag) {
                    continue;
                }
                if !crate::app::controller::library::source_folders::folder_filter_accepts(
                    &entry.relative_path,
                    folder_selection.as_ref(),
                    folder_negated.as_ref(),
                    root_mode,
                ) {
                    continue;
                }
                visible.push(index);
            }
            apply_sort_for_similar(controller, &mut visible, sort_mode, &similar);
            controller.ui_cache.browser.pipeline.sorted_rows = visible;
            controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
        }
        return visible_result_from_sorted(controller, focused_index, loaded_index);
    }

    if let Some(query) = query {
        controller.ensure_search_scores(&query);
        let score_fingerprint = hash_value(&(
            filtered_fingerprint,
            hash_value(&query),
            controller.ui_cache.browser.search.scores.len(),
        ));
        if controller.ui_cache.browser.pipeline.scored_fingerprint != Some(score_fingerprint) {
            let filtered_rows = controller.ui_cache.browser.pipeline.filtered_rows.clone();
            let mut scored = Vec::with_capacity(filtered_rows.len());
            for index in filtered_rows {
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

        let sorted_fingerprint = hash_value(&(
            controller.ui_cache.browser.pipeline.scored_fingerprint,
            sort_key(sort_mode),
        ));
        if controller.ui_cache.browser.pipeline.sorted_fingerprint != Some(sorted_fingerprint) {
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
                sort_visible_by_playback_age(
                    controller,
                    &mut visible,
                    sort_mode == SampleBrowserSort::PlaybackAgeAsc,
                );
            }
            controller.ui_cache.browser.pipeline.sorted_rows = visible;
            controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
        }

        return visible_result_from_sorted(controller, focused_index, loaded_index);
    }

    let sorted_fingerprint = hash_value(&(filtered_fingerprint, sort_key(sort_mode)));
    if controller.ui_cache.browser.pipeline.sorted_fingerprint != Some(sorted_fingerprint) {
        let mut visible = controller.ui_cache.browser.pipeline.filtered_rows.clone();
        if matches!(
            sort_mode,
            SampleBrowserSort::PlaybackAgeAsc | SampleBrowserSort::PlaybackAgeDesc
        ) {
            sort_visible_by_playback_age(
                controller,
                &mut visible,
                sort_mode == SampleBrowserSort::PlaybackAgeAsc,
            );
        }
        controller.ui_cache.browser.pipeline.sorted_rows = visible;
        controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
    }

    visible_result_from_sorted(controller, focused_index, loaded_index)
}

/// Ensure stage-A cached base rows and triage partitions are current.
fn ensure_base_stage(controller: &mut AppController) {
    let source_id = controller.selection_state.ctx.selected_source.clone();
    let source_revision = controller
        .current_source()
        .and_then(|source| controller.database_for(&source).ok())
        .and_then(|db| db.get_revision().ok());
    let fingerprint = BaseStageFingerprint {
        source_id,
        source_revision,
        entries_len: controller.wav_entries_len(),
    };
    if controller
        .ui_cache
        .browser
        .pipeline
        .base_fingerprint
        .as_ref()
        == Some(&fingerprint)
    {
        return;
    }

    let mut base_rows = Vec::with_capacity(controller.wav_entries_len());
    let mut trash_rows = Vec::new();
    let mut neutral_rows = Vec::new();
    let mut keep_rows = Vec::new();
    let _ = controller.for_each_wav_entry(|index, entry| {
        base_rows.push(index);
        if entry.tag.is_trash() {
            trash_rows.push(index);
        } else if entry.tag.is_keep() {
            keep_rows.push(index);
        } else {
            neutral_rows.push(index);
        }
    });
    controller.ui_cache.browser.pipeline.base_rows = base_rows;
    controller.ui_cache.browser.pipeline.trash_rows = trash_rows;
    controller.ui_cache.browser.pipeline.neutral_rows = neutral_rows;
    controller.ui_cache.browser.pipeline.keep_rows = keep_rows;
    controller.ui_cache.browser.pipeline.base_fingerprint = Some(fingerprint);
    controller.ui_cache.browser.pipeline.filtered_fingerprint = None;
    controller.ui_cache.browser.pipeline.scored_fingerprint = None;
    controller.ui_cache.browser.pipeline.sorted_fingerprint = None;
}

/// Return the visible rows output from the sorted stage cache.
fn visible_result_from_sorted(
    controller: &mut AppController,
    focused_index: Option<usize>,
    loaded_index: Option<usize>,
) -> (VisibleRows, Option<usize>, Option<usize>) {
    let visible = controller.ui_cache.browser.pipeline.sorted_rows.clone();
    let selected_visible =
        focused_index.and_then(|index| visible.iter().position(|row| *row == index));
    let loaded_visible =
        loaded_index.and_then(|index| visible.iter().position(|row| *row == index));
    (VisibleRows::List(visible), selected_visible, loaded_visible)
}

/// Apply explicit sort policy for similarity-query result rows.
fn apply_sort_for_similar(
    controller: &mut AppController,
    visible: &mut [usize],
    sort_mode: SampleBrowserSort,
    similar: &crate::app::state::SimilarQuery,
) {
    match sort_mode {
        SampleBrowserSort::Similarity => {
            let mut lookup = vec![None; controller.wav_entries_len()];
            for (&index, &score) in similar.indices.iter().zip(similar.scores.iter()) {
                if index < lookup.len() {
                    lookup[index] = Some(score);
                }
            }
            visible.sort_by(|a, b| {
                let a_score = lookup
                    .get(*a)
                    .and_then(|score| *score)
                    .unwrap_or(f32::NEG_INFINITY);
                let b_score = lookup
                    .get(*b)
                    .and_then(|score| *score)
                    .unwrap_or(f32::NEG_INFINITY);
                b_score
                    .partial_cmp(&a_score)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| a.cmp(b))
            });
            if let Some(anchor) = similar.anchor_index
                && let Some(pos) = visible.iter().position(|index| *index == anchor)
            {
                visible.rotate_right(visible.len().saturating_sub(pos));
            }
        }
        SampleBrowserSort::PlaybackAgeAsc => {
            sort_visible_by_playback_age(controller, visible, true);
        }
        SampleBrowserSort::PlaybackAgeDesc => {
            sort_visible_by_playback_age(controller, visible, false);
        }
        SampleBrowserSort::ListOrder => {
            visible.sort_unstable();
        }
    }
}

/// Return whether an entry tag matches the active triage/rating filters.
fn filter_accepts(
    filter: TriageFlagFilter,
    rating_filter: &std::collections::BTreeSet<i8>,
    tag: crate::sample_sources::Rating,
) -> bool {
    let triage_ok = match filter {
        TriageFlagFilter::All => true,
        TriageFlagFilter::Keep => tag.is_keep(),
        TriageFlagFilter::Trash => tag.is_trash(),
        TriageFlagFilter::Untagged => tag.is_neutral(),
    };
    let rating_ok = rating_filter.is_empty() || rating_filter.contains(&tag.val());
    triage_ok && rating_ok
}

/// Sort visible row indices by playback age then by absolute index.
fn sort_visible_by_playback_age(
    controller: &mut AppController,
    visible: &mut [usize],
    ascending: bool,
) {
    visible.sort_by(|a, b| {
        let a_key = controller
            .wav_entry(*a)
            .and_then(|entry| entry.last_played_at)
            .unwrap_or(i64::MIN);
        let b_key = controller
            .wav_entry(*b)
            .and_then(|entry| entry.last_played_at)
            .unwrap_or(i64::MIN);
        let order = if ascending {
            a_key.cmp(&b_key)
        } else {
            b_key.cmp(&a_key)
        };
        order.then_with(|| a.cmp(b))
    });
}

/// Hash any value into a compact stage fingerprint.
fn hash_value<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Convert root-folder mode into a stable scalar for cache keys.
fn root_mode_key(mode: crate::app::state::RootFolderFilterMode) -> u8 {
    match mode {
        crate::app::state::RootFolderFilterMode::AllDescendants => 0,
        crate::app::state::RootFolderFilterMode::RootOnly => 1,
    }
}

/// Convert triage-filter enum into a stable scalar for cache keys.
fn filter_key(filter: TriageFlagFilter) -> u8 {
    match filter {
        TriageFlagFilter::All => 0,
        TriageFlagFilter::Keep => 1,
        TriageFlagFilter::Trash => 2,
        TriageFlagFilter::Untagged => 3,
    }
}

/// Convert browser-sort enum into a stable scalar for cache keys.
fn sort_key(sort: SampleBrowserSort) -> u8 {
    match sort {
        SampleBrowserSort::ListOrder => 0,
        SampleBrowserSort::Similarity => 1,
        SampleBrowserSort::PlaybackAgeAsc => 2,
        SampleBrowserSort::PlaybackAgeDesc => 3,
    }
}

/// Hash a similarity query payload for stage cache invalidation.
fn similarity_fingerprint(query: &crate::app::state::SimilarQuery) -> u64 {
    hash_value(&(
        &query.sample_id,
        &query.label,
        &query.indices,
        query
            .scores
            .iter()
            .map(|score| score.to_bits())
            .collect::<Vec<u32>>(),
        query.anchor_index,
    ))
}
