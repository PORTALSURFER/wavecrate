use super::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::sync::Arc;

use crate::app::controller::library::wavs::search_scoring::{
    ScoreCandidateResult, score_query_candidates,
};

use super::query_score_cache::BrowserQueryScoreCacheScope;

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
        let path_fingerprint = self.browser_search_path_fingerprint();
        let search_cache = &self.ui_cache.browser.search;
        if self.ui_cache.browser.search.source_id == source_id
            && self.ui_cache.browser.search.query == query
            && self.ui_cache.browser.search.scores.len() == entries_len
            && search_cache.path_fingerprint == path_fingerprint
        {
            return;
        }
        let path_changed = self
            .ui_cache
            .browser
            .search
            .sync_path_fingerprint(path_fingerprint);
        let scope = BrowserQueryScoreCacheScope {
            source_id: source_id.clone(),
            path_fingerprint,
        };
        if let Some(cached) =
            self.ui_cache
                .browser
                .search
                .promote_exact_query(&scope, query, entries_len)
        {
            self.ui_cache.browser.search.source_id = cached.scope.source_id.clone();
            self.ui_cache.browser.search.path_fingerprint = cached.scope.path_fingerprint;
            self.ui_cache.browser.search.query.clone_from(&cached.query);
            self.ui_cache.browser.search.scores = cached.scores.clone();
            return;
        }
        if self.ui_cache.browser.search.source_id != source_id
            || self.ui_cache.browser.search.query != query
            || self.ui_cache.browser.search.scores.len() != entries_len
            || path_changed
        {
            self.compute_search_scores(query, source_id, path_fingerprint, entries_len);
        }
    }

    fn compute_search_scores(
        &mut self,
        query: &str,
        source_id: Option<SourceId>,
        path_fingerprint: u64,
        entries_len: usize,
    ) {
        self.ui_cache.browser.search.source_id = source_id;
        self.ui_cache.browser.search.path_fingerprint = path_fingerprint;
        self.ui_cache.browser.search.query.clear();
        self.ui_cache.browser.search.query.push_str(query);
        let Some(source_id) = self.selection_state.ctx.selected_source.clone() else {
            return;
        };
        let label_path_fingerprint = self.browser_label_path_fingerprint();
        self.ensure_browser_label_cache(&source_id, entries_len, label_path_fingerprint);
        let prefix_scope = BrowserQueryScoreCacheScope {
            source_id: Some(source_id),
            path_fingerprint,
        };
        let prefix_cache =
            self.ui_cache
                .browser
                .search
                .reusable_prefix_query(&prefix_scope, query, entries_len);
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
        self.ui_cache.browser.search.store_query(
            BrowserQueryScoreCacheScope {
                source_id: self.ui_cache.browser.search.source_id.clone(),
                path_fingerprint: self.ui_cache.browser.search.path_fingerprint,
            },
            matched_indices,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
    use crate::sample_sources::Rating;

    #[test]
    fn ensure_search_scores_recomputes_after_same_length_reorder() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("noise.wav", Rating::NEUTRAL),
            sample_entry("abc.wav", Rating::NEUTRAL),
        ]);
        controller.set_browser_search("abc");

        let stale_source_id = controller.ui_cache.browser.search.source_id.clone();
        let stale_query = controller.ui_cache.browser.search.query.clone();
        let stale_scores = controller.ui_cache.browser.search.scores.clone();
        let stale_query_score_cache = controller.ui_cache.browser.search.query_score_cache.clone();
        let stale_path_fingerprint = controller.ui_cache.browser.search.path_fingerprint;

        controller.set_wav_entries_for_tests(vec![
            sample_entry("abc.wav", Rating::NEUTRAL),
            sample_entry("noise.wav", Rating::NEUTRAL),
        ]);
        controller.ui_cache.browser.labels.clear();
        controller.ui_cache.browser.search.source_id = stale_source_id;
        controller.ui_cache.browser.search.query = stale_query;
        controller.ui_cache.browser.search.scores = stale_scores;
        controller.ui_cache.browser.search.query_score_cache = stale_query_score_cache;
        controller.ui_cache.browser.search.path_fingerprint = stale_path_fingerprint;

        controller.rebuild_browser_lists();

        let visible = (0..controller.visible_browser_len())
            .filter_map(|row| controller.visible_browser_index(row))
            .collect::<Vec<_>>();
        assert_eq!(visible, vec![0]);
        assert_ne!(
            controller.ui_cache.browser.search.path_fingerprint,
            stale_path_fingerprint
        );
        assert_eq!(
            controller.ui_cache.browser.search.query_score_cache[0]
                .matched_indices
                .as_ref(),
            &[0]
        );
    }
}
