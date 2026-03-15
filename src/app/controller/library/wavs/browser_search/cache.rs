//! Search-score caching and label-fill helpers for the sample browser.

use super::*;
use crate::app::view_model;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::sync::Arc;

use crate::app::controller::library::wavs::search_scoring::{
    QueryScoreCacheEntry, ScoreCandidateResult, promote_exact_query_score_cache_entry,
    reusable_prefix_query_score_cache_entry, score_query_candidates, store_query_score_cache_entry,
};

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
    /// Return the active trimmed browser query, when non-empty.
    pub(crate) fn active_search_query(&self) -> Option<&str> {
        let query = self.ui.browser.search.search_query.trim();
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

    /// Return a display label for one wav entry, filling the retained label cache on demand.
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
}
