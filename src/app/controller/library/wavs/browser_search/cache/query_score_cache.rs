use super::*;
use std::sync::Arc;

use crate::app::controller::library::wavs::search_scoring::{
    QueryScoreCacheEntry, promote_exact_query_score_cache_entry,
    reusable_prefix_query_score_cache_entry, store_query_score_cache_entry,
};

/// Source/path-snapshot scope for one synchronous browser query-score cache entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BrowserQueryScoreCacheScope {
    /// Selected source associated with the cached score vector.
    pub(super) source_id: Option<SourceId>,
    /// Ordered-path fingerprint for the entry snapshot used during scoring.
    pub(super) path_fingerprint: u64,
}

/// Cached score payload for a specific source/query/path snapshot combination.
pub(super) type BrowserQueryScoreCacheEntry = QueryScoreCacheEntry<BrowserQueryScoreCacheScope>;

/// Cache state for browser search scoring and sort scratch buffers.
pub(crate) struct BrowserSearchCache {
    pub(super) source_id: Option<SourceId>,
    pub(super) query: String,
    pub(super) path_fingerprint: u64,
    pub(crate) scores: Arc<[Option<i64>]>,
    pub(crate) scratch: Vec<(usize, i64)>,
    pub(super) query_score_cache: Vec<BrowserQueryScoreCacheEntry>,
    pub(super) max_cached_queries: usize,
}

impl BrowserSearchCache {
    /// Construct an empty search cache.
    pub(crate) fn new() -> Self {
        Self {
            source_id: None,
            query: String::new(),
            path_fingerprint: 0,
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
        self.path_fingerprint = 0;
        self.scores = Arc::from([]);
        self.scratch.clear();
        self.query_score_cache.clear();
    }

    /// Refresh the ordered-path fingerprint and drop stale query scores when it changes.
    pub(super) fn sync_path_fingerprint(&mut self, path_fingerprint: u64) -> bool {
        if self.path_fingerprint == path_fingerprint {
            return false;
        }
        self.path_fingerprint = path_fingerprint;
        self.query_score_cache.clear();
        true
    }

    pub(super) fn promote_exact_query(
        &mut self,
        scope: &BrowserQueryScoreCacheScope,
        query: &str,
        entries_len: usize,
    ) -> Option<BrowserQueryScoreCacheEntry> {
        promote_exact_query_score_cache_entry(
            &mut self.query_score_cache,
            scope,
            query,
            entries_len,
        )
    }

    pub(super) fn reusable_prefix_query(
        &self,
        scope: &BrowserQueryScoreCacheScope,
        query: &str,
        entries_len: usize,
    ) -> Option<BrowserQueryScoreCacheEntry> {
        reusable_prefix_query_score_cache_entry(&self.query_score_cache, scope, query, entries_len)
    }

    pub(super) fn store_query(
        &mut self,
        scope: BrowserQueryScoreCacheScope,
        matched_indices: Arc<[usize]>,
    ) {
        store_query_score_cache_entry(
            &mut self.query_score_cache,
            self.max_cached_queries,
            scope,
            self.query.clone(),
            self.scores.clone(),
            matched_indices,
        );
    }
}

impl Default for BrowserSearchCache {
    /// Build a search cache with bounded recent-query score retention.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::library::wavs::search_scoring::QueryScoreCacheEntry;

    #[test]
    fn sync_path_fingerprint_change_clears_cached_query_scores() {
        let mut cache = BrowserSearchCache {
            path_fingerprint: 11,
            query_score_cache: vec![QueryScoreCacheEntry {
                scope: BrowserQueryScoreCacheScope {
                    source_id: Some(SourceId::new()),
                    path_fingerprint: 11,
                },
                query: String::from("kick"),
                scores: Arc::from([Some(1)]),
                matched_indices: Arc::from([0]),
            }],
            ..BrowserSearchCache::default()
        };

        assert!(cache.sync_path_fingerprint(22));
        assert_eq!(cache.path_fingerprint, 22);
        assert!(cache.query_score_cache.is_empty());
        assert!(!cache.sync_path_fingerprint(22));
    }
}
