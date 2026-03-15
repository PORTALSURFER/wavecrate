use super::*;
use crate::app::state::{SampleBrowserSort, TriageFlagFilter, VisibleRows};
use std::sync::Arc;

/// Shared stage helper functions for sort/filter/hash operations.
mod helpers;

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
    /// Fingerprint for the cached folder-filter acceptance map.
    folder_accepts_fingerprint: Option<u64>,
    /// Cached folder-filter acceptance by absolute wav-entry index.
    folder_accepts_by_index: Vec<bool>,
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
    /// Sorted visible absolute entry indices, retained for cheap sharing.
    sorted_rows: Arc<[usize]>,
}

impl BrowserPipelineCache {
    /// Drop all staged fingerprints and vectors.
    pub(crate) fn invalidate(&mut self) {
        self.base_fingerprint = None;
        self.base_rows.clear();
        self.trash_rows.clear();
        self.neutral_rows.clear();
        self.keep_rows.clear();
        self.folder_accepts_fingerprint = None;
        self.folder_accepts_by_index.clear();
        self.filtered_fingerprint = None;
        self.filtered_rows.clear();
        self.scored_fingerprint = None;
        self.scored_rows.clear();
        self.sorted_fingerprint = None;
        self.sorted_rows = Vec::new().into();
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
    let similar_query = controller.ui.browser.search.similar_query.clone();
    let sort_mode = controller.ui.browser.search.sort;
    let filter = controller.ui.browser.search.filter;
    let rating_filter = controller.ui.browser.search.rating_filter.clone();
    let rating_filter_hash = helpers::hash_value(&rating_filter);
    let folder_selection = controller.folder_selection_for_filter().cloned();
    let folder_negated = controller.folder_negation_for_filter().cloned();
    let root_mode = controller
        .root_folder_filter_mode_for_filter()
        .unwrap_or_default();
    let folder_hash = crate::app::controller::library::source_folders::folder_filter_fingerprint(
        folder_selection.as_ref(),
        folder_negated.as_ref(),
        root_mode,
    );
    let has_folder_filters = crate::app::controller::library::source_folders::folder_filters_active(
        folder_selection.as_ref(),
        folder_negated.as_ref(),
        root_mode,
    );
    ensure_folder_acceptance_stage(
        controller,
        folder_selection.as_ref(),
        folder_negated.as_ref(),
        root_mode,
        folder_hash,
        has_folder_filters,
    );

    if query.is_none()
        && similar_query.is_none()
        && sort_mode == SampleBrowserSort::ListOrder
        && filter == TriageFlagFilter::All
        && controller.ui.browser.search.rating_filter.is_empty()
        && !has_folder_filters
    {
        let total = controller.wav_entries_len();
        return (VisibleRows::All { total }, focused_index, loaded_index);
    }

    let base_fingerprint_hash =
        helpers::hash_value(&controller.ui_cache.browser.pipeline.base_fingerprint);
    let filtered_fingerprint = helpers::hash_value(&(
        base_fingerprint_hash,
        helpers::filter_key(filter),
        rating_filter_hash,
        folder_hash,
    ));
    if controller.ui_cache.browser.pipeline.filtered_fingerprint != Some(filtered_fingerprint) {
        let base_len = controller.ui_cache.browser.pipeline.base_rows.len();
        let mut filtered_rows = Vec::with_capacity(base_len);
        for row in 0..base_len {
            let index = controller.ui_cache.browser.pipeline.base_rows[row];
            let Some(entry) = controller.wav_entry(index) else {
                continue;
            };
            if !helpers::filter_accepts(filter, &rating_filter, entry.tag, entry.locked) {
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

    if let Some(similar) = similar_query {
        let sorted_fingerprint = helpers::hash_value(&(
            filtered_fingerprint,
            helpers::sort_key(sort_mode),
            helpers::similarity_fingerprint(&similar),
        ));
        if controller.ui_cache.browser.pipeline.sorted_fingerprint != Some(sorted_fingerprint) {
            let mut visible = Vec::with_capacity(similar.indices.len());
            for index in similar.indices.iter().copied() {
                let Some(entry) = controller.wav_entry(index) else {
                    continue;
                };
                if !helpers::filter_accepts(filter, &rating_filter, entry.tag, entry.locked) {
                    continue;
                }
                if !folder_accepts(controller, index) {
                    continue;
                }
                visible.push(index);
            }
            helpers::apply_sort_for_similar(controller, &mut visible, sort_mode, &similar);
            controller.ui_cache.browser.pipeline.sorted_rows = visible.into();
            controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
        }
        return visible_result_from_sorted(controller, focused_index, loaded_index);
    }

    if let Some(query) = query {
        controller.ensure_search_scores(&query);
        let score_fingerprint = helpers::hash_value(&(
            filtered_fingerprint,
            helpers::hash_value(&query),
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
                helpers::sort_visible_by_playback_age(
                    controller,
                    &mut visible,
                    sort_mode == SampleBrowserSort::PlaybackAgeAsc,
                );
            }
            controller.ui_cache.browser.pipeline.sorted_rows = visible.into();
            controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
        }

        return visible_result_from_sorted(controller, focused_index, loaded_index);
    }

    let sorted_fingerprint =
        helpers::hash_value(&(filtered_fingerprint, helpers::sort_key(sort_mode)));
    if controller.ui_cache.browser.pipeline.sorted_fingerprint != Some(sorted_fingerprint) {
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
    controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_fingerprint = None;
    controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_by_index
        .clear();
    controller.ui_cache.browser.pipeline.filtered_fingerprint = None;
    controller.ui_cache.browser.pipeline.scored_fingerprint = None;
    controller.ui_cache.browser.pipeline.sorted_fingerprint = None;
}

/// Ensure folder-filter acceptance values are cached for the current base snapshot.
fn ensure_folder_acceptance_stage(
    controller: &mut AppController,
    folder_selection: Option<&std::collections::BTreeSet<std::path::PathBuf>>,
    folder_negated: Option<&std::collections::BTreeSet<std::path::PathBuf>>,
    root_mode: crate::app::state::RootFolderFilterMode,
    folder_hash: u64,
    has_folder_filters: bool,
) {
    let base_fingerprint_hash =
        helpers::hash_value(&controller.ui_cache.browser.pipeline.base_fingerprint);
    let fingerprint = helpers::hash_value(&(base_fingerprint_hash, folder_hash));
    let entries_len = controller.wav_entries_len();
    if controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_fingerprint
        == Some(fingerprint)
        && controller
            .ui_cache
            .browser
            .pipeline
            .folder_accepts_by_index
            .len()
            == entries_len
    {
        return;
    }

    let accepts = if has_folder_filters {
        let relative_paths: Vec<_> = (0..entries_len)
            .map(|index| {
                controller
                    .wav_entry(index)
                    .map(|entry| entry.relative_path.clone())
            })
            .collect();
        crate::app::controller::library::source_folders::build_folder_filter_acceptance_map(
            relative_paths.iter().map(|path| path.as_deref()),
            folder_selection,
            folder_negated,
            root_mode,
        )
    } else {
        vec![true; entries_len]
    };

    controller.ui_cache.browser.pipeline.folder_accepts_by_index = accepts;
    controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_fingerprint = Some(fingerprint);
}

/// Return cached folder-filter acceptance for an absolute wav-entry index.
fn folder_accepts(controller: &AppController, index: usize) -> bool {
    controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_by_index
        .get(index)
        .copied()
        .unwrap_or(false)
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
