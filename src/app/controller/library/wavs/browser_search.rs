use super::search_scoring::{
    QueryScoreCacheEntry, ScoreCandidateResult, promote_exact_query_score_cache_entry,
    reusable_prefix_query_score_cache_entry, score_query_candidates, store_query_score_cache_entry,
};
use super::*;
use crate::app::state::SampleBrowserSort;
use crate::app::view_model;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::sync::{Arc, OnceLock};

/// Environment override for the browser-search offload threshold.
const SEARCH_OFFLOAD_THRESHOLD_ENV: &str = "SEMPAL_BROWSER_SEARCH_OFFLOAD_THRESHOLD";
/// Environment override for enabling/disabling async browser search for UI interactions.
const SEARCH_ASYNC_PIPELINE_ENV: &str = "SEMPAL_BROWSER_ASYNC_PIPELINE";
/// Default wav-entry count threshold above which search work offloads to jobs.
const DEFAULT_SEARCH_OFFLOAD_THRESHOLD: usize = 5_000;

/// Cached score payload for a specific source/query combination.
type BrowserQueryScoreCacheEntry = QueryScoreCacheEntry<Option<SourceId>>;

/// Cache state for browser search scoring and sort scratch buffers.
pub(crate) struct BrowserSearchCache {
    source_id: Option<SourceId>,
    query: String,
    pub(crate) scores: Arc<[Option<i64>]>,
    scratch: Vec<(usize, i64)>,
    query_score_cache: Vec<BrowserQueryScoreCacheEntry>,
    max_cached_queries: usize,
}

impl BrowserSearchCache {
    /// Construct an empty search cache.
    pub(crate) fn new() -> Self {
        Self {
            source_id: None,
            query: String::new(),
            scores: Arc::from([]),
            scratch: Vec::new(),
            query_score_cache: Vec::new(),
            max_cached_queries: 6,
        }
    }

    /// Clear all cached search inputs, scores, and query history.
    pub(crate) fn invalidate(&mut self) {
        self.source_id = None;
        self.query.clear();
        self.scores = Arc::from([]);
        self.scratch.clear();
        self.query_score_cache.clear();
    }
}

impl Default for BrowserSearchCache {
    /// Build a search cache with bounded recent-query score retention.
    fn default() -> Self {
        Self::new()
    }
}

impl AppController {
    /// Return `true` when browser search should run through the async job path.
    pub(crate) fn should_offload_search(&self) -> bool {
        self.wav_entries_len() > browser_search_offload_threshold()
    }

    /// Return `true` when browser interactions should dispatch async search jobs.
    pub(crate) fn should_dispatch_browser_search_async(&self) -> bool {
        browser_async_pipeline_enabled()
    }

    /// Return the active trimmed browser query, when non-empty.
    pub(crate) fn active_search_query(&self) -> Option<&str> {
        let query = self.ui.browser.search_query.trim();
        if query.is_empty() { None } else { Some(query) }
    }

    /// Ensure fuzzy-match scores exist for the query against current visible entries.
    pub(crate) fn ensure_search_scores(&mut self, query: &str) {
        let entries_len = self.wav_entries_len();
        let source_id = self.selection_state.ctx.selected_source.clone();
        if self.ui_cache.browser.search.source_id == source_id
            && self.ui_cache.browser.search.query == query
            && self.ui_cache.browser.search.scores.len() == entries_len
        {
            return;
        }
        if let Some(cached) = promote_exact_query_score_cache_entry(
            &mut self.ui_cache.browser.search.query_score_cache,
            &source_id,
            query,
            entries_len,
        ) {
            self.ui_cache.browser.search.source_id = cached.scope.clone();
            self.ui_cache.browser.search.query.clone_from(&cached.query);
            self.ui_cache.browser.search.scores = cached.scores.clone();
            return;
        }
        if self.ui_cache.browser.search.source_id != source_id
            || self.ui_cache.browser.search.query != query
            || self.ui_cache.browser.search.scores.len() != entries_len
        {
            self.ui_cache.browser.search.source_id = source_id;
            self.ui_cache.browser.search.query.clear();
            self.ui_cache.browser.search.query.push_str(query);
            let Some(source_id) = self.selection_state.ctx.selected_source.clone() else {
                return;
            };
            let needs_labels = self
                .ui_cache
                .browser
                .labels
                .get(&source_id)
                .map(|cached| cached.len() != entries_len)
                .unwrap_or(true);
            if needs_labels {
                self.ui_cache
                    .browser
                    .labels
                    .insert(source_id.clone(), vec![String::new(); entries_len]);
            }
            let prefix_cache = reusable_prefix_query_score_cache_entry(
                &self.ui_cache.browser.search.query_score_cache,
                &Some(source_id.clone()),
                query,
                entries_len,
            );
            let candidate_indices = prefix_cache
                .as_ref()
                .map(|cached| cached.matched_indices.as_ref());
            let mut new_scores = vec![None; entries_len];
            let matcher = SkimMatcherV2::default();
            let matched_indices = score_query_candidates(
                &mut new_scores,
                candidate_indices,
                entries_len,
                |index, _| {
                    let score = self
                        .label_for_ref(index)
                        .filter(|label| !label.is_empty())
                        .and_then(|label| matcher.fuzzy_match(label, query));
                    ScoreCandidateResult::Continue(score)
                },
            )
            .expect("synchronous search scoring cannot cancel");
            self.ui_cache.browser.search.scores = Arc::from(new_scores);
            store_query_score_cache_entry(
                &mut self.ui_cache.browser.search.query_score_cache,
                self.ui_cache.browser.search.max_cached_queries,
                self.ui_cache.browser.search.source_id.clone(),
                self.ui_cache.browser.search.query.clone(),
                self.ui_cache.browser.search.scores.clone(),
                matched_indices,
            );
        }
    }

    pub(crate) fn label_for_ref(&mut self, index: usize) -> Option<&str> {
        let source_id = self.selection_state.ctx.selected_source.clone()?;
        let needs_labels = self
            .ui_cache
            .browser
            .labels
            .get(&source_id)
            .map(|cached| cached.len() != self.wav_entries_len())
            .unwrap_or(true);
        if needs_labels {
            self.ui_cache.browser.labels.insert(
                source_id.clone(),
                vec![String::new(); self.wav_entries_len()],
            );
        }
        let needs_fill = self
            .ui_cache
            .browser
            .labels
            .get(&source_id)
            .and_then(|labels| labels.get(index))
            .is_some_and(|label| label.is_empty());
        if needs_fill {
            let entry = self.wav_entry(index)?;
            let label = view_model::sample_display_label(&entry.relative_path);
            if let Some(labels) = self.ui_cache.browser.labels.get_mut(&source_id)
                && index < labels.len()
            {
                labels[index] = label;
            }
        }
        self.ui_cache
            .browser
            .labels
            .get(&source_id)
            .and_then(|labels| labels.get(index))
            .map(|label| label.as_str())
    }

    pub(crate) fn dispatch_search_job(&mut self) {
        let Some(source) = self.current_source() else {
            self.mark_browser_search_projection_revision_dirty();
            self.ui.browser.search_busy = false;
            return;
        };
        self.ui.browser.latest_search_request_id =
            self.ui.browser.latest_search_request_id.wrapping_add(1);
        let request_id = self.ui.browser.latest_search_request_id;
        let query = self.ui.browser.search_query.clone();
        let filter = self.ui.browser.filter;
        let rating_filter = self.ui.browser.rating_filter.clone();
        let sort = self.ui.browser.sort;
        let similar_query = self.ui.browser.similar_query.clone();
        let folder_selection = self.folder_selection_for_filter().cloned();
        let folder_negated = self.folder_negation_for_filter().cloned();
        let root_mode = self
            .root_folder_filter_mode_for_filter()
            .unwrap_or_default();

        self.mark_browser_search_projection_revision_dirty();
        self.ui.browser.search_busy = true;
        self.runtime
            .jobs
            .send_search_job(crate::app::controller::jobs::SearchJob {
                request_id,
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                query,
                filter,
                rating_filter,
                sort,
                similar_query,
                folder_selection,
                folder_negated,
                root_mode,
            });
    }
}

pub(crate) fn set_browser_filter(controller: &mut AppController, filter: TriageFlagFilter) {
    if controller.ui.browser.filter != filter {
        controller.ui.browser.filter = filter;
        controller.mark_browser_search_projection_revision_dirty();
        if controller.should_dispatch_browser_search_async() {
            controller.dispatch_search_job();
        } else {
            controller.rebuild_browser_lists();
        }
    }
}

/// Update the browser rating filter selection.
pub(crate) fn set_browser_rating_filter(controller: &mut AppController, level: i8, additive: bool) {
    if !(-3..=4).contains(&level) {
        return;
    }
    let mut changed = false;
    if additive {
        if controller.ui.browser.rating_filter.contains(&level) {
            controller.ui.browser.rating_filter.remove(&level);
        } else {
            controller.ui.browser.rating_filter.insert(level);
        }
        changed = true;
    } else if controller.ui.browser.rating_filter.len() != 1
        || !controller.ui.browser.rating_filter.contains(&level)
    {
        controller.ui.browser.rating_filter.clear();
        controller.ui.browser.rating_filter.insert(level);
        changed = true;
    }
    if changed {
        controller.mark_browser_search_projection_revision_dirty();
        if controller.should_dispatch_browser_search_async() {
            controller.dispatch_search_job();
        } else {
            controller.rebuild_browser_lists();
        }
    }
}

/// Replace the active browser rating filter set and refresh visible rows when it changes.
fn replace_browser_rating_filter(
    controller: &mut AppController,
    levels: impl IntoIterator<Item = i8>,
) {
    let next_filter = levels
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    if controller.ui.browser.rating_filter == next_filter {
        return;
    }
    controller.ui.browser.rating_filter = next_filter;
    controller.mark_browser_search_projection_revision_dirty();
    if controller.should_dispatch_browser_search_async() {
        controller.dispatch_search_job();
    } else {
        controller.rebuild_browser_lists();
    }
}

/// Return every valid rating-filter level except the clicked chip level.
fn inverted_browser_rating_filter_levels(level: i8) -> Option<std::collections::BTreeSet<i8>> {
    const ALL_BROWSER_RATING_FILTER_LEVELS: [i8; 8] = [-3, -2, -1, 0, 1, 2, 3, 4];
    if !ALL_BROWSER_RATING_FILTER_LEVELS.contains(&level) {
        return None;
    }
    Some(
        ALL_BROWSER_RATING_FILTER_LEVELS
            .into_iter()
            .filter(|candidate| *candidate != level)
            .collect(),
    )
}

/// Invert one browser rating-filter chip into every other valid filter level.
pub(crate) fn invert_browser_rating_filter(controller: &mut AppController, level: i8) {
    let Some(levels) = inverted_browser_rating_filter_levels(level) else {
        return;
    };
    if controller.ui.browser.rating_filter == levels {
        clear_browser_rating_filter(controller);
    } else {
        replace_browser_rating_filter(controller, levels);
    }
}

/// Clear all browser rating filters.
pub(crate) fn clear_browser_rating_filter(controller: &mut AppController) {
    if controller.ui.browser.rating_filter.is_empty() {
        return;
    }
    controller.ui.browser.rating_filter.clear();
    controller.mark_browser_search_projection_revision_dirty();
    if controller.should_dispatch_browser_search_async() {
        controller.dispatch_search_job();
    } else {
        controller.rebuild_browser_lists();
    }
}

pub(crate) fn set_browser_sort(controller: &mut AppController, sort: SampleBrowserSort) {
    if controller.ui.browser.sort != sort {
        controller.ui.browser.sort = sort;
        if sort != SampleBrowserSort::Similarity {
            controller.ui.browser.similarity_sort_follow_loaded = false;
        }
        controller.mark_browser_search_projection_revision_dirty();
        if controller.should_dispatch_browser_search_async() {
            controller.dispatch_search_job();
        } else {
            controller.rebuild_browser_lists();
        }
    }
}

pub(crate) fn focus_browser_search(controller: &mut AppController) {
    controller.focus_browser_context();
    if controller.ui.browser.search_focus_requested {
        return;
    }
    controller.ui.browser.search_focus_requested = true;
    controller.mark_browser_search_projection_revision_dirty();
}

/// Clear browser-search focus while leaving the current query text intact.
pub(crate) fn blur_browser_search(controller: &mut AppController) {
    if !controller.ui.browser.search_focus_requested {
        return;
    }
    controller.ui.browser.search_focus_requested = false;
    controller.mark_browser_search_projection_revision_dirty();
}

pub(crate) fn set_browser_search(controller: &mut AppController, query: impl Into<String>) {
    let query = query.into();
    if controller.ui.browser.search_query == query {
        return;
    }
    controller.ui.browser.search_query = query;
    controller.mark_browser_search_projection_revision_dirty();
    controller.ui.browser.similar_query = None;
    controller.ui.browser.sort = SampleBrowserSort::ListOrder;
    controller.ui.browser.similarity_sort_follow_loaded = false;
    if controller.should_dispatch_browser_search_async() {
        controller.dispatch_search_job();
    } else {
        controller.rebuild_browser_lists();
    }
}

/// Resolve the wav-entry threshold for switching browser search to async jobs.
fn browser_search_offload_threshold() -> usize {
    /// Cached parsed offload threshold for browser search jobs.
    static OFFLOAD_THRESHOLD: OnceLock<usize> = OnceLock::new();
    *OFFLOAD_THRESHOLD.get_or_init(|| {
        std::env::var(SEARCH_OFFLOAD_THRESHOLD_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .filter(|threshold| *threshold > 0)
            .unwrap_or(DEFAULT_SEARCH_OFFLOAD_THRESHOLD)
    })
}

/// Resolve whether browser interaction paths should always use async search.
///
/// This defaults to `true` for runtime builds and `false` under libtest so
/// tests keep deterministic immediate list updates.
fn browser_async_pipeline_enabled() -> bool {
    #[cfg(test)]
    {
        false
    }
    #[cfg(not(test))]
    {
        /// Cached parsed async pipeline override for browser interactions.
        static ASYNC_PIPELINE_ENABLED: OnceLock<bool> = OnceLock::new();
        *ASYNC_PIPELINE_ENABLED.get_or_init(|| {
            std::env::var(SEARCH_ASYNC_PIPELINE_ENV)
                .ok()
                .as_deref()
                .and_then(crate::env_flags::parse_env_bool)
                .unwrap_or(true)
        })
    }
}
