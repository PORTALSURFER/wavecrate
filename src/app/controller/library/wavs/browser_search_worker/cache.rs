//! Worker-local source/query caches for browser search processing.

use super::*;

pub(super) struct CompactSearchEntry {
    pub(super) display_label: Box<str>,
    pub(super) relative_path: Box<str>,
    pub(super) tag: Rating,
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
    pub(super) source_id: Option<String>,
    pub(super) source_root: Option<PathBuf>,
    pub(super) revision: u64,
    pub(super) db_stamp: Option<DbFileStamp>,
    pub(super) query_score_cache: Vec<WorkerQueryScoreCacheEntry>,
    pub(super) max_cached_queries: usize,
    pub(super) folder_accept_cache: Vec<WorkerFolderAcceptCacheEntry>,
    pub(super) max_cached_folder_filters: usize,
    pub(super) triage_cache: Option<WorkerTriageCacheEntry>,
}

impl Default for SearchWorkerCache {
    /// Initialize an empty worker cache with bounded recent-query score retention.
    fn default() -> Self {
        Self {
            db: None,
            entries: None,
            source_id: None,
            source_root: None,
            revision: 0,
            db_stamp: None,
            query_score_cache: Vec::new(),
            max_cached_queries: 6,
            folder_accept_cache: Vec::new(),
            max_cached_folder_filters: 4,
            triage_cache: None,
        }
    }
}

/// Cached query score vector keyed by source revision and query text.
pub(super) struct WorkerQueryScoreCacheEntry {
    pub(super) source_id: String,
    pub(super) revision: u64,
    pub(super) query: String,
    pub(super) scores: Arc<[Option<i64>]>,
}

/// Cached folder-filter acceptance vector for one source revision + folder filter shape.
pub(super) struct WorkerFolderAcceptCacheEntry {
    pub(super) source_id: String,
    pub(super) revision: u64,
    pub(super) folder_filter_hash: u64,
    pub(super) accepts: Arc<[bool]>,
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
