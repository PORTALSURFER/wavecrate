use super::*;
use crate::app::state::SampleBrowserSort;
use crate::app::view_model;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

/// Cached score payload for a specific source/query combination.
#[derive(Clone)]
struct QueryScoreCacheEntry {
    /// Source associated with the score vector.
    source_id: Option<SourceId>,
    /// Exact query string associated with the score vector.
    query: String,
    /// Score vector aligned to absolute entry indices.
    scores: Vec<Option<i64>>,
}

/// Cache state for browser search scoring and sort scratch buffers.
pub(crate) struct BrowserSearchCache {
    source_id: Option<SourceId>,
    query: String,
    pub(crate) scores: Vec<Option<i64>>,
    scratch: Vec<(usize, i64)>,
    query_score_cache: Vec<QueryScoreCacheEntry>,
    max_cached_queries: usize,
    pub(crate) matcher: SkimMatcherV2,
}

impl BrowserSearchCache {
    /// Construct an empty search cache.
    pub(crate) fn new() -> Self {
        Self {
            source_id: None,
            query: String::new(),
            scores: Vec::new(),
            scratch: Vec::new(),
            query_score_cache: Vec::new(),
            max_cached_queries: 6,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Clear all cached search inputs, scores, and query history.
    pub(crate) fn invalidate(&mut self) {
        self.source_id = None;
        self.query.clear();
        self.scores.clear();
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
    pub(crate) fn should_offload_search(&self) -> bool {
        self.wav_entries_len() > 5000
    }

    #[allow(dead_code)]
    fn browser_filter_accepts(&self, tag: crate::sample_sources::Rating) -> bool {
        let triage_ok = match self.ui.browser.filter {
            TriageFlagFilter::All => true,
            TriageFlagFilter::Keep => tag.is_keep(),
            TriageFlagFilter::Trash => tag.is_trash(),
            TriageFlagFilter::Untagged => tag.is_neutral(),
        };
        let rating_ok = self.ui.browser.rating_filter.is_empty()
            || self.ui.browser.rating_filter.contains(&tag.val());
        triage_ok && rating_ok
    }

    /// Return the active trimmed browser query, when non-empty.
    pub(crate) fn active_search_query(&self) -> Option<&str> {
        let query = self.ui.browser.search_query.trim();
        if query.is_empty() { None } else { Some(query) }
    }

    /// Ensure fuzzy-match scores exist for the query against current visible entries.
    pub(crate) fn ensure_search_scores(&mut self, query: &str) {
        let source_id = self.selection_state.ctx.selected_source.clone();
        if self.ui_cache.browser.search.source_id == source_id
            && self.ui_cache.browser.search.query == query
            && self.ui_cache.browser.search.scores.len() == self.wav_entries_len()
        {
            return;
        }
        if let Some(cached_index) = self
            .ui_cache
            .browser
            .search
            .query_score_cache
            .iter()
            .position(|entry| {
                entry.source_id == source_id
                    && entry.query == query
                    && entry.scores.len() == self.wav_entries_len()
            })
        {
            let cached = self
                .ui_cache
                .browser
                .search
                .query_score_cache
                .remove(cached_index);
            self.ui_cache.browser.search.source_id = cached.source_id.clone();
            self.ui_cache.browser.search.query.clone_from(&cached.query);
            self.ui_cache.browser.search.scores = cached.scores;
            self.ui_cache.browser.search.query_score_cache.insert(
                0,
                QueryScoreCacheEntry {
                    source_id: self.ui_cache.browser.search.source_id.clone(),
                    query: self.ui_cache.browser.search.query.clone(),
                    scores: self.ui_cache.browser.search.scores.clone(),
                },
            );
            return;
        }
        if self.ui_cache.browser.search.source_id != source_id
            || self.ui_cache.browser.search.query != query
            || self.ui_cache.browser.search.scores.len() != self.wav_entries_len()
        {
            self.ui_cache.browser.search.source_id = source_id;
            self.ui_cache.browser.search.query.clear();
            self.ui_cache.browser.search.query.push_str(query);
            self.ui_cache.browser.search.scores.clear();
            self.ui_cache
                .browser
                .search
                .scores
                .resize(self.wav_entries_len(), None);

            let Some(source_id) = self.selection_state.ctx.selected_source.clone() else {
                return;
            };
            let needs_labels = self
                .ui_cache
                .browser
                .labels
                .get(&source_id)
                .map(|cached| cached.len() != self.wav_entries_len())
                .unwrap_or(true);
            if needs_labels {
                self.ui_cache
                    .browser
                    .labels
                    .insert(source_id.clone(), Vec::new());
            }
            let mut label_strings: Vec<Option<String>> = Vec::with_capacity(self.wav_entries_len());
            for idx in 0..self.wav_entries_len() {
                let lbl = self.label_for_ref(idx).map(|s| s.to_string());
                label_strings.push(lbl);
            }

            let mut new_scores: Vec<Option<i64>> = Vec::with_capacity(label_strings.len());
            for lbl_opt in label_strings {
                if let Some(lbl_str) = lbl_opt {
                    let score = self
                        .ui_cache
                        .browser
                        .search
                        .matcher
                        .fuzzy_match(&lbl_str, query);
                    new_scores.push(score);
                } else {
                    new_scores.push(None);
                }
            }
            self.ui_cache.browser.search.scores = new_scores;
            self.ui_cache.browser.search.query_score_cache.insert(
                0,
                QueryScoreCacheEntry {
                    source_id: self.ui_cache.browser.search.source_id.clone(),
                    query: self.ui_cache.browser.search.query.clone(),
                    scores: self.ui_cache.browser.search.scores.clone(),
                },
            );
            self.ui_cache
                .browser
                .search
                .query_score_cache
                .truncate(self.ui_cache.browser.search.max_cached_queries);
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
            return;
        };
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

        self.ui.browser.search_busy = true;
        self.runtime
            .jobs
            .send_search_job(crate::app::controller::jobs::SearchJob {
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
        controller.rebuild_browser_lists();
    }
}

/// Update the browser rating filter selection.
pub(crate) fn set_browser_rating_filter(controller: &mut AppController, level: i8, additive: bool) {
    if !(-3..=3).contains(&level) {
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
        controller.rebuild_browser_lists();
    }
}

/// Clear all browser rating filters.
pub(crate) fn clear_browser_rating_filter(controller: &mut AppController) {
    if controller.ui.browser.rating_filter.is_empty() {
        return;
    }
    controller.ui.browser.rating_filter.clear();
    controller.rebuild_browser_lists();
}

pub(crate) fn set_browser_sort(controller: &mut AppController, sort: SampleBrowserSort) {
    if controller.ui.browser.sort != sort {
        controller.ui.browser.sort = sort;
        if sort != SampleBrowserSort::Similarity {
            controller.ui.browser.similarity_sort_follow_loaded = false;
        }
        controller.rebuild_browser_lists();
    }
}

pub(crate) fn focus_browser_search(controller: &mut AppController) {
    controller.ui.browser.search_focus_requested = true;
    controller.focus_browser_context();
}

pub(crate) fn set_browser_search(controller: &mut AppController, query: impl Into<String>) {
    let query = query.into();
    if controller.ui.browser.search_query == query {
        return;
    }
    controller.ui.browser.search_query = query;
    controller.ui.browser.similar_query = None;
    controller.ui.browser.sort = SampleBrowserSort::ListOrder;
    controller.ui.browser.similarity_sort_follow_loaded = false;
    controller.rebuild_browser_lists();
}
