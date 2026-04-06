//! Worker-local source/query caches for browser search processing.

use super::super::search_scoring::QueryScoreCacheEntry;
use super::*;
use std::collections::HashMap;

pub(super) struct CompactSearchEntry {
    pub(super) display_label: Box<str>,
    pub(super) relative_path: Box<str>,
    pub(super) tag: Rating,
    pub(super) locked: bool,
    pub(super) last_played_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DbFileStamp {
    pub(super) modified: Option<SystemTime>,
    pub(super) len: u64,
}

impl DbFileStamp {
    pub(super) fn from_path(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        let modified = metadata.modified().ok();
        Some(Self {
            modified,
            len: metadata.len(),
        })
    }
}

pub(super) struct SearchWorkerCache {
    pub(super) db: Option<crate::sample_sources::SourceDatabase>,
    pub(super) entries: Option<Vec<CompactSearchEntry>>,
    pub(super) entry_lookup: HashMap<String, usize>,
    pub(super) source_id: Option<String>,
    pub(super) source_root: Option<PathBuf>,
    pub(super) revision: u64,
    pub(super) paths_revision: u64,
    pub(super) path_fingerprint: u64,
    pub(super) db_stamp: Option<DbFileStamp>,
    pub(super) query_score_cache: Vec<WorkerQueryScoreCacheEntry>,
    pub(super) max_cached_queries: usize,
    pub(super) folder_accept_cache: Vec<WorkerFolderAcceptCacheEntry>,
    pub(super) max_cached_folder_filters: usize,
    pub(super) filter_stage_cache: Vec<WorkerFilterStageCacheEntry>,
    pub(super) max_cached_filter_stages: usize,
    pub(super) playback_age_token_caches: Vec<WorkerPlaybackAgeTokenCache>,
    pub(super) triage_cache: Option<WorkerTriageCacheEntry>,
    pub(super) score_scratch: Vec<Option<i64>>,
    pub(super) similar_lookup_scratch: Vec<Option<f32>>,
    pub(super) scored_index_scratch: Vec<(usize, i64)>,
}

impl Default for SearchWorkerCache {
    /// Initialize an empty worker cache with bounded recent-query score retention.
    fn default() -> Self {
        Self {
            db: None,
            entries: None,
            entry_lookup: HashMap::new(),
            source_id: None,
            source_root: None,
            revision: 0,
            paths_revision: 0,
            path_fingerprint: 0,
            db_stamp: None,
            query_score_cache: Vec::new(),
            max_cached_queries: 6,
            folder_accept_cache: Vec::new(),
            max_cached_folder_filters: 4,
            filter_stage_cache: Vec::new(),
            max_cached_filter_stages: 6,
            playback_age_token_caches: Vec::new(),
            triage_cache: None,
            score_scratch: Vec::new(),
            similar_lookup_scratch: Vec::new(),
            scored_index_scratch: Vec::new(),
        }
    }
}

impl SearchWorkerCache {
    /// Ensure score scratch has `len` elements and return added element capacity.
    pub(super) fn prepare_score_scratch(&mut self, len: usize) -> usize {
        let added = reserve_growth(&mut self.score_scratch, len);
        self.score_scratch.clear();
        self.score_scratch.resize(len, None);
        added
    }

    /// Ensure similarity-lookup scratch has `len` elements and return added capacity.
    pub(super) fn prepare_similar_lookup_scratch(&mut self, len: usize) -> usize {
        let added = reserve_growth(&mut self.similar_lookup_scratch, len);
        self.similar_lookup_scratch.clear();
        self.similar_lookup_scratch.resize(len, None);
        added
    }

    /// Ensure scored-index scratch can hold `capacity` entries and return added capacity.
    pub(super) fn prepare_scored_index_scratch(&mut self, capacity: usize) -> usize {
        let added = reserve_growth(&mut self.scored_index_scratch, capacity);
        self.scored_index_scratch.clear();
        added
    }
}

/// Reserve vector capacity up to `target_capacity` and return added element capacity.
fn reserve_growth<T>(buffer: &mut Vec<T>, target_capacity: usize) -> usize {
    let before = buffer.capacity();
    if before < target_capacity {
        buffer.reserve(target_capacity.saturating_sub(before));
    }
    buffer.capacity().saturating_sub(before)
}

/// Source/path-snapshot scope for one worker query-score cache entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WorkerQueryScoreCacheScope {
    pub(super) source_id: String,
    pub(super) path_fingerprint: u64,
}

/// Cached query score vector keyed by source path snapshot and query text.
pub(super) type WorkerQueryScoreCacheEntry = QueryScoreCacheEntry<WorkerQueryScoreCacheScope>;

/// Cached folder-filter acceptance vector for one source revision + folder filter shape.
pub(super) struct WorkerFolderAcceptCacheEntry {
    pub(super) source_id: String,
    pub(super) revision: u64,
    pub(super) folder_filter_hash: u64,
    pub(super) accepts: Arc<[bool]>,
}

/// Cached composed filter acceptance keyed by source revision and filter shape.
pub(super) struct WorkerFilterStageCacheEntry {
    pub(super) source_id: String,
    pub(super) revision: u64,
    pub(super) filter_hash: u64,
    pub(super) accepts: Arc<[bool]>,
    pub(super) rows: Arc<[usize]>,
}

/// Cached next playback-age rollover token for one worker revision and chip set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WorkerPlaybackAgeTokenCache {
    pub(super) revision: u64,
    pub(super) filter_hash: u64,
    pub(super) token: Option<i64>,
}

/// Cached triage partitions for one source revision.
pub(super) struct WorkerTriageCacheEntry {
    pub(super) source_id: String,
    pub(super) revision: u64,
    pub(super) len: usize,
    pub(super) trash: Arc<[usize]>,
    pub(super) neutral: Arc<[usize]>,
    pub(super) keep: Arc<[usize]>,
}

/// Shared triage partitions in source-list index order.
pub(super) type TriagePartitions = (Arc<[usize]>, Arc<[usize]>, Arc<[usize]>);

#[cfg(test)]
/// Worker-cache scratch-buffer helper tests.
mod tests {
    use super::*;

    #[test]
    /// Preparing score scratch should clear stale values and match requested length.
    fn prepare_score_scratch_clears_and_resizes() {
        let mut cache = SearchWorkerCache {
            score_scratch: vec![Some(1), Some(2)],
            ..SearchWorkerCache::default()
        };

        let _ = cache.prepare_score_scratch(4);

        assert_eq!(cache.score_scratch.len(), 4);
        assert!(cache.score_scratch.iter().all(Option::is_none));
    }

    #[test]
    /// Preparing scored-index scratch should retain capacity and clear prior items.
    fn prepare_scored_index_scratch_reuses_capacity() {
        let mut cache = SearchWorkerCache {
            scored_index_scratch: vec![(1, 10), (2, 20)],
            ..SearchWorkerCache::default()
        };
        let initial_capacity = cache.scored_index_scratch.capacity();

        let _ = cache.prepare_scored_index_scratch(1);

        assert!(cache.scored_index_scratch.is_empty());
        assert!(cache.scored_index_scratch.capacity() >= initial_capacity);
    }
}
