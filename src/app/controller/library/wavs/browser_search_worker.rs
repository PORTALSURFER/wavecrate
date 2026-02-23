use crate::app::controller::jobs::{SearchJob, SearchResult};
use crate::app::state::{SampleBrowserSort, TriageFlagFilter, VisibleRows};
use crate::sample_sources::Rating;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::SystemTime;
use tracing::warn;

struct CompactSearchEntry {
    display_label: Box<str>,
    relative_path: Box<str>,
    tag: Rating,
    last_played_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DbFileStamp {
    modified: Option<SystemTime>,
    len: u64,
}

impl DbFileStamp {
    fn from_path(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        let modified = metadata.modified().ok();
        Some(Self {
            modified,
            len: metadata.len(),
        })
    }
}

struct SearchWorkerCache {
    db: Option<crate::sample_sources::SourceDatabase>,
    entries: Option<Vec<CompactSearchEntry>>,
    source_id: Option<String>,
    source_root: Option<PathBuf>,
    revision: u64,
    db_stamp: Option<DbFileStamp>,
    query_score_cache: Vec<WorkerQueryScoreCacheEntry>,
    max_cached_queries: usize,
    folder_accept_cache: Vec<WorkerFolderAcceptCacheEntry>,
    max_cached_folder_filters: usize,
    triage_cache: Option<WorkerTriageCacheEntry>,
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
struct WorkerQueryScoreCacheEntry {
    source_id: String,
    revision: u64,
    query: String,
    scores: Vec<Option<i64>>,
}

/// Cached folder-filter acceptance vector for one source revision + folder filter shape.
struct WorkerFolderAcceptCacheEntry {
    source_id: String,
    revision: u64,
    folder_filter_hash: u64,
    accepts: Arc<[bool]>,
}

/// Cached triage partitions for one source revision.
struct WorkerTriageCacheEntry {
    source_id: String,
    revision: u64,
    len: usize,
    trash: Vec<usize>,
    neutral: Vec<usize>,
    keep: Vec<usize>,
}

#[derive(Default)]
struct SearchJobQueueState {
    pending: Option<SearchJob>,
    poisoned_recovered: bool,
    shutdown: bool,
}

/// Latest-only queue for browser search jobs.
struct SearchJobQueue {
    state: Mutex<SearchJobQueueState>,
    ready: Condvar,
}

impl SearchJobQueue {
    fn new() -> Self {
        Self {
            state: Mutex::new(SearchJobQueueState::default()),
            ready: Condvar::new(),
        }
    }

    fn send(&self, job: SearchJob) {
        let mut state = self.lock_state();
        if state.shutdown {
            return;
        }
        state.pending = Some(job);
        self.ready.notify_one();
    }

    fn shutdown(&self) {
        let mut state = self.lock_state();
        state.shutdown = true;
        state.pending = None;
        self.ready.notify_all();
    }

    fn take_blocking(&self) -> Option<SearchJob> {
        let mut state = self.lock_state();
        loop {
            if state.shutdown {
                return None;
            }
            if let Some(job) = state.pending.take() {
                return Some(job);
            }
            state = self.wait_ready(state);
        }
    }

    #[cfg(test)]
    fn try_take(&self) -> Option<SearchJob> {
        let mut state = self.lock_state();
        state.pending.take()
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, SearchJobQueueState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => self.recover_state("lock", poisoned),
        }
    }

    fn wait_ready<'a>(
        &self,
        guard: std::sync::MutexGuard<'a, SearchJobQueueState>,
    ) -> std::sync::MutexGuard<'a, SearchJobQueueState> {
        self.ready
            .wait(guard)
            .unwrap_or_else(|poisoned| self.recover_state("condvar", poisoned))
    }

    fn recover_state<'a>(
        &self,
        context: &'static str,
        poisoned: std::sync::PoisonError<std::sync::MutexGuard<'a, SearchJobQueueState>>,
    ) -> std::sync::MutexGuard<'a, SearchJobQueueState> {
        let mut guard = poisoned.into_inner();
        if !guard.poisoned_recovered {
            warn!("Search job queue {context} poisoned; recovering and clearing pending job.");
            guard.pending = None;
            guard.poisoned_recovered = true;
        }
        guard
    }
}

/// Sender handle for coalesced search jobs.
#[derive(Clone)]
pub(crate) struct SearchJobSender {
    queue: Arc<SearchJobQueue>,
}

impl SearchJobSender {
    /// Replace any pending search job with the latest request.
    pub(crate) fn send(&self, job: SearchJob) {
        self.queue.send(job);
    }
}

/// Join handle and shutdown signal for the browser search worker thread.
pub(crate) struct SearchWorkerHandle {
    queue: Arc<SearchJobQueue>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl SearchWorkerHandle {
    /// Signal the worker thread to exit and wait for it to finish.
    pub(crate) fn shutdown(&mut self) {
        self.queue.shutdown();
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Spawn a background worker that processes the latest pending search job.
/// Returns the sender, result channel, and a shutdown handle.
pub(crate) fn spawn_search_worker() -> (SearchJobSender, Receiver<SearchResult>, SearchWorkerHandle)
{
    let queue = Arc::new(SearchJobQueue::new());
    let sender = SearchJobSender {
        queue: Arc::clone(&queue),
    };
    let (result_tx, result_rx) = std::sync::mpsc::channel::<SearchResult>();
    let queue_worker = Arc::clone(&queue);
    let handle = thread::spawn(move || {
        let matcher = SkimMatcherV2::default();
        let mut cache = SearchWorkerCache::default();
        while let Some(job) = queue_worker.take_blocking() {
            let result = process_search_job(job, &matcher, &mut cache);
            let _ = result_tx.send(result);
        }
    });
    (
        sender,
        result_rx,
        SearchWorkerHandle {
            queue,
            join_handle: Some(handle),
        },
    )
}

fn process_search_job(
    job: SearchJob,
    matcher: &SkimMatcherV2,
    cache: &mut SearchWorkerCache,
) -> SearchResult {
    let job_source_id_str = job.source_id.as_str().to_string();
    let db_path = crate::sample_sources::database_path_for(&job.source_root);
    let db_stamp = DbFileStamp::from_path(&db_path);

    let must_reopen = cache.db.is_none()
        || cache.source_id.as_ref() != Some(&job_source_id_str)
        || cache.source_root.as_ref() != Some(&job.source_root)
        || cache.db_stamp.as_ref() != db_stamp.as_ref();

    if must_reopen {
        match crate::sample_sources::SourceDatabase::open_read_only(&job.source_root) {
            Ok(db) => {
                cache.db = Some(db);
                cache.entries = None;
                cache.revision = 0;
                cache.source_id = Some(job_source_id_str.clone());
                cache.source_root = Some(job.source_root.clone());
                cache.db_stamp = db_stamp;
                cache.query_score_cache.clear();
                cache.folder_accept_cache.clear();
                cache.triage_cache = None;
            }
            Err(_) => {
                cache.db = None;
                cache.entries = None;
                cache.revision = 0;
                cache.source_id = Some(job_source_id_str);
                cache.source_root = Some(job.source_root.clone());
                cache.db_stamp = db_stamp;
                cache.query_score_cache.clear();
                cache.folder_accept_cache.clear();
                cache.triage_cache = None;
                return empty_search_result(job);
            }
        }
    }

    let db = match cache.db.as_ref() {
        Some(db) => db,
        None => return empty_search_result(job),
    };

    let revision = db.get_revision().unwrap_or(0);
    let must_reload = cache.entries.is_none() || cache.revision != revision;

    if must_reload {
        match db.list_files() {
            Ok(loaded_entries) => {
                let compact_entries: Vec<CompactSearchEntry> = loaded_entries
                    .into_iter()
                    .map(|e| {
                        let relative_path = e.relative_path.to_string_lossy().to_string();
                        let display_label =
                            crate::app::view_model::sample_display_label(&e.relative_path);

                        CompactSearchEntry {
                            display_label: display_label.into_boxed_str(),
                            relative_path: relative_path.into_boxed_str(),
                            tag: e.tag,
                            last_played_at: e.last_played_at,
                        }
                    })
                    .collect();
                cache.entries = Some(compact_entries);
                cache.revision = revision;
                cache.query_score_cache.clear();
                cache.folder_accept_cache.clear();
                cache.triage_cache = None;
            }
            Err(_) => {
                cache.entries = None;
                cache.query_score_cache.clear();
                cache.folder_accept_cache.clear();
                cache.triage_cache = None;
                return empty_search_result(job);
            }
        }
    }

    let has_query = !job.query.is_empty();
    let entries_len = cache.entries.as_ref().map(Vec::len).unwrap_or(0);
    let has_folder_filters = crate::app::controller::library::source_folders::folder_filters_active(
        job.folder_selection.as_ref(),
        job.folder_negated.as_ref(),
        job.root_mode,
    );
    let mut scores = vec![None; entries_len];

    if has_query {
        let Some(entries) = cache.entries.as_ref() else {
            return empty_search_result(job);
        };
        if let Some(index) = cache.query_score_cache.iter().position(|cached| {
            cached.source_id == job_source_id_str
                && cached.revision == cache.revision
                && cached.query == job.query
                && cached.scores.len() == entries_len
        }) {
            let cached = cache.query_score_cache.remove(index);
            scores = cached.scores;
            cache.query_score_cache.insert(
                0,
                WorkerQueryScoreCacheEntry {
                    source_id: job_source_id_str.clone(),
                    revision: cache.revision,
                    query: job.query.clone(),
                    scores: scores.clone(),
                },
            );
        } else {
            for (index, entry) in entries.iter().enumerate() {
                scores[index] = matcher.fuzzy_match(&entry.display_label, &job.query);
            }
            cache.query_score_cache.insert(
                0,
                WorkerQueryScoreCacheEntry {
                    source_id: job_source_id_str.clone(),
                    revision: cache.revision,
                    query: job.query.clone(),
                    scores: scores.clone(),
                },
            );
            cache.query_score_cache.truncate(cache.max_cached_queries);
        }
    }

    let (trash, neutral, keep) =
        triage_partitions_for_revision(cache, &job_source_id_str, cache.revision);
    if !has_query
        && !has_folder_filters
        && job.filter == TriageFlagFilter::All
        && job.similar_query.is_none()
        && job.sort == SampleBrowserSort::ListOrder
        && job.rating_filter.is_empty()
    {
        return SearchResult {
            request_id: job.request_id,
            source_id: job.source_id,
            query: job.query,
            visible: VisibleRows::All { total: entries_len },
            trash,
            neutral,
            keep,
            scores,
        };
    }

    let folder_accepts = folder_accepts_for_job(
        cache,
        &job,
        &job_source_id_str,
        cache.revision,
        has_folder_filters,
    );
    let Some(entries) = cache.entries.as_ref() else {
        return empty_search_result(job);
    };
    let mut visible = Vec::new();

    if let Some(similar) = &job.similar_query {
        for index in similar.indices.iter().copied() {
            if let Some(entry) = entries.get(index)
                && filter_accepts_tag(job.filter, &job.rating_filter, entry.tag)
                && folder_accepts_index(folder_accepts.as_ref(), index)
            {
                visible.push(index);
            }
        }

        match job.sort {
            SampleBrowserSort::Similarity => {
                let mut score_lookup = vec![None; entries.len()];
                for (&index, &score) in similar.indices.iter().zip(similar.scores.iter()) {
                    if index < score_lookup.len() {
                        score_lookup[index] = Some(score);
                    }
                }
                visible.sort_by(|a: &usize, b: &usize| {
                    let a_score = score_lookup
                        .get(*a)
                        .and_then(|s| *s)
                        .unwrap_or(f32::NEG_INFINITY);
                    let b_score = score_lookup
                        .get(*b)
                        .and_then(|s| *s)
                        .unwrap_or(f32::NEG_INFINITY);
                    b_score
                        .partial_cmp(&a_score)
                        .unwrap_or(Ordering::Equal)
                        .then_with(|| a.cmp(b))
                });

                if let Some(anchor) = similar.anchor_index
                    && let Some(entry) = entries.get(anchor)
                    && filter_accepts_tag(job.filter, &job.rating_filter, entry.tag)
                    && folder_accepts_index(folder_accepts.as_ref(), anchor)
                {
                    if let Some(pos) = visible.iter().position(|i| *i == anchor) {
                        visible.remove(pos);
                    }
                    visible.insert(0, anchor);
                }
            }
            SampleBrowserSort::PlaybackAgeAsc => {
                sort_visible_by_playback_age(entries, &mut visible, true);
            }
            SampleBrowserSort::PlaybackAgeDesc => {
                sort_visible_by_playback_age(entries, &mut visible, false);
            }
            SampleBrowserSort::ListOrder => {
                visible.sort_unstable();
            }
        }
    } else {
        let mut scratch = Vec::with_capacity(entries.len().min(1024));
        for (index, entry) in entries.iter().enumerate() {
            if !filter_accepts_tag(job.filter, &job.rating_filter, entry.tag)
                || !folder_accepts_index(folder_accepts.as_ref(), index)
            {
                continue;
            }
            if has_query {
                if let Some(score) = scores[index] {
                    scratch.push((index, score));
                }
            } else {
                visible.push(index);
            }
        }

        if has_query {
            scratch.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            visible = scratch.into_iter().map(|(index, _)| index).collect();
        }
        match job.sort {
            SampleBrowserSort::PlaybackAgeAsc => {
                sort_visible_by_playback_age(entries, &mut visible, true);
            }
            SampleBrowserSort::PlaybackAgeDesc => {
                sort_visible_by_playback_age(entries, &mut visible, false);
            }
            _ => {}
        }
    }

    SearchResult {
        request_id: job.request_id,
        source_id: job.source_id,
        query: job.query,
        visible: VisibleRows::List(visible.into()),
        trash,
        neutral,
        keep,
        scores,
    }
}

/// Return whether a tag passes the active triage + rating filter settings.
fn filter_accepts_tag(
    filter: TriageFlagFilter,
    rating_filter: &std::collections::BTreeSet<i8>,
    tag: Rating,
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

/// Return whether a row index passes the cached folder-filter acceptance map.
fn folder_accepts_index(folder_accepts: Option<&Arc<[bool]>>, index: usize) -> bool {
    folder_accepts
        .map(|accepts| accepts.get(index).copied().unwrap_or(false))
        .unwrap_or(true)
}

/// Resolve the cached folder-filter acceptance map for the current job.
fn folder_accepts_for_job(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    source_id: &str,
    revision: u64,
    has_folder_filters: bool,
) -> Option<Arc<[bool]>> {
    if !has_folder_filters {
        return None;
    }
    let entries_len = cache.entries.as_ref().map(Vec::len).unwrap_or(0);
    let folder_filter_hash = folder_filter_hash_for_job(job);
    if let Some(index) = cache.folder_accept_cache.iter().position(|cached| {
        cached.source_id == source_id
            && cached.revision == revision
            && cached.folder_filter_hash == folder_filter_hash
            && cached.accepts.len() == entries_len
    }) {
        let cached = cache.folder_accept_cache.remove(index);
        cache.folder_accept_cache.insert(0, cached);
        return cache
            .folder_accept_cache
            .first()
            .map(|cached| Arc::clone(&cached.accepts));
    }

    let accepts = cache
        .entries
        .as_ref()
        .map(|entries| build_folder_accepts(entries, job))
        .unwrap_or_default();
    cache.folder_accept_cache.insert(
        0,
        WorkerFolderAcceptCacheEntry {
            source_id: source_id.to_string(),
            revision,
            folder_filter_hash,
            accepts: accepts.into(),
        },
    );
    cache
        .folder_accept_cache
        .truncate(cache.max_cached_folder_filters);
    cache
        .folder_accept_cache
        .first()
        .map(|cached| Arc::clone(&cached.accepts))
}

/// Build folder-filter acceptance values for all entries in source order.
fn build_folder_accepts(entries: &[CompactSearchEntry], job: &SearchJob) -> Vec<bool> {
    let mut accepts = vec![false; entries.len()];
    for (index, entry) in entries.iter().enumerate() {
        let path = Path::new(entry.relative_path.as_ref());
        accepts[index] = crate::app::controller::library::source_folders::folder_filter_accepts(
            path,
            job.folder_selection.as_ref(),
            job.folder_negated.as_ref(),
            job.root_mode,
        );
    }
    accepts
}

/// Resolve revision-keyed triage partitions, rebuilding only when revision changes.
fn triage_partitions_for_revision(
    cache: &mut SearchWorkerCache,
    source_id: &str,
    revision: u64,
) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    let entries = match cache.entries.as_ref() {
        Some(entries) => entries,
        None => return (Vec::new(), Vec::new(), Vec::new()),
    };
    let needs_rebuild = cache
        .triage_cache
        .as_ref()
        .map(|cached| {
            cached.source_id != source_id
                || cached.revision != revision
                || cached.len != entries.len()
        })
        .unwrap_or(true);
    if needs_rebuild {
        let mut trash = Vec::new();
        let mut neutral = Vec::new();
        let mut keep = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            if entry.tag.is_trash() {
                trash.push(index);
            } else if entry.tag.is_keep() {
                keep.push(index);
            } else {
                neutral.push(index);
            }
        }
        cache.triage_cache = Some(WorkerTriageCacheEntry {
            source_id: source_id.to_string(),
            revision,
            len: entries.len(),
            trash,
            neutral,
            keep,
        });
    }
    if let Some(cached) = cache.triage_cache.as_ref() {
        return (
            cached.trash.clone(),
            cached.neutral.clone(),
            cached.keep.clone(),
        );
    }
    (Vec::new(), Vec::new(), Vec::new())
}

/// Hash a folder-filter payload into a stable worker cache key.
fn folder_filter_hash_for_job(job: &SearchJob) -> u64 {
    hash_value(&(
        job.folder_selection.as_ref(),
        job.folder_negated.as_ref(),
        root_mode_key(job.root_mode),
    ))
}

/// Convert root-folder mode into a compact scalar for hashing.
fn root_mode_key(mode: crate::app::state::RootFolderFilterMode) -> u8 {
    match mode {
        crate::app::state::RootFolderFilterMode::AllDescendants => 0,
        crate::app::state::RootFolderFilterMode::RootOnly => 1,
    }
}

/// Hash an arbitrary value for worker cache fingerprints.
fn hash_value<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn empty_search_result(job: SearchJob) -> SearchResult {
    SearchResult {
        request_id: job.request_id,
        source_id: job.source_id,
        query: job.query,
        visible: VisibleRows::List(Vec::new().into()),
        trash: Vec::new(),
        neutral: Vec::new(),
        keep: Vec::new(),
        scores: Vec::new(),
    }
}

fn sort_visible_by_playback_age(
    entries: &[CompactSearchEntry],
    visible: &mut [usize],
    ascending: bool,
) {
    visible.sort_by(|a, b| {
        let a_key = entries
            .get(*a)
            .and_then(|entry| entry.last_played_at)
            .unwrap_or(i64::MIN);
        let b_key = entries
            .get(*b)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_sources::SourceId;
    use crate::sample_sources::WavEntry;
    use std::collections::BTreeSet;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn test_compact_search_entry() {
        let entries = vec![
            WavEntry {
                relative_path: std::path::PathBuf::from("kits/drums/kick.wav"),
                file_size: 100,
                modified_ns: 1,
                content_hash: None,
                tag: Rating::NEUTRAL,
                looped: false,
                missing: false,
                last_played_at: None,
            },
            WavEntry {
                relative_path: std::path::PathBuf::from("kits/drums/snare.wav"),
                file_size: 110,
                modified_ns: 2,
                content_hash: None,
                tag: Rating::NEUTRAL,
                looped: false,
                missing: false,
                last_played_at: None,
            },
        ];

        let compacts: Vec<CompactSearchEntry> = entries
            .into_iter()
            .map(|e| {
                let relative_path = e.relative_path.to_string_lossy().to_string();
                let display_label = crate::app::view_model::sample_display_label(&e.relative_path);
                CompactSearchEntry {
                    display_label: display_label.into_boxed_str(),
                    relative_path: relative_path.into_boxed_str(),
                    tag: e.tag,
                    last_played_at: e.last_played_at,
                }
            })
            .collect();

        assert_eq!(compacts.len(), 2);
        assert_eq!(compacts[0].display_label.as_ref(), "kick");
        assert_eq!(compacts[1].display_label.as_ref(), "snare");
        assert_eq!(compacts[0].relative_path.as_ref(), "kits/drums/kick.wav");
    }

    #[test]
    fn latest_search_job_replaces_pending() {
        let queue = Arc::new(SearchJobQueue::new());
        let sender = SearchJobSender {
            queue: Arc::clone(&queue),
        };

        let first = make_search_job("first", "one");
        let second = make_search_job("second", "two");

        sender.send(first);
        sender.send(second);

        let pending = queue.try_take().expect("expected pending search job");
        assert_eq!(pending.query, "second");
        assert!(queue.try_take().is_none());
    }

    #[test]
    fn search_queue_recovers_from_poison() {
        let queue = Arc::new(SearchJobQueue::new());
        let queue_for_panic = Arc::clone(&queue);
        let _ = std::panic::catch_unwind(move || {
            let _guard = queue_for_panic.state.lock().expect("queue lock failed");
            panic!("poison search job queue");
        });

        let (tx, rx) = mpsc::channel();
        let queue_for_worker = Arc::clone(&queue);
        let handle = thread::spawn(move || {
            let job = queue_for_worker
                .take_blocking()
                .expect("expected job after recovery");
            tx.send(job.query).expect("send result");
        });

        queue.send(make_search_job("recovered", "root"));

        let received = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("search job never received");
        assert_eq!(received, "recovered");
        handle.join().expect("worker thread panicked");
    }

    #[test]
    fn search_queue_shutdown_unblocks() {
        let queue = Arc::new(SearchJobQueue::new());
        let (tx, rx) = mpsc::channel();
        let queue_for_worker = Arc::clone(&queue);
        let handle = thread::spawn(move || {
            let result = queue_for_worker.take_blocking();
            tx.send(result.is_none()).expect("send shutdown");
        });
        queue.shutdown();
        let shutdown = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("shutdown result");
        assert!(shutdown);
        handle.join().expect("worker thread panicked");
    }

    #[test]
    /// Revision-keyed triage cache should reuse cached partitions without changing outputs.
    fn triage_partitions_are_cached_by_revision() {
        let mut cache = SearchWorkerCache {
            entries: Some(vec![
                compact_entry("a.wav", Rating::TRASH_3),
                compact_entry("b.wav", Rating::NEUTRAL),
                compact_entry("c.wav", Rating::KEEP_3),
            ]),
            ..SearchWorkerCache::default()
        };
        let first = triage_partitions_for_revision(&mut cache, "source", 7);
        let second = triage_partitions_for_revision(&mut cache, "source", 7);

        assert_eq!(first.0, vec![0]);
        assert_eq!(first.1, vec![1]);
        assert_eq!(first.2, vec![2]);
        assert_eq!(first, second);
        assert!(cache.triage_cache.is_some());
    }

    #[test]
    /// Worker folder acceptance cache must invalidate when root-mode semantics change.
    fn folder_filter_hash_changes_with_root_mode() {
        let mut all = make_search_job("q", "root");
        all.folder_selection = Some(BTreeSet::from([PathBuf::from("")]));
        all.root_mode = crate::app::state::RootFolderFilterMode::AllDescendants;

        let mut root_only = make_search_job("q", "root");
        root_only.folder_selection = Some(BTreeSet::from([PathBuf::from("")]));
        root_only.root_mode = crate::app::state::RootFolderFilterMode::RootOnly;

        assert_ne!(
            folder_filter_hash_for_job(&all),
            folder_filter_hash_for_job(&root_only)
        );
    }

    /// Build a compact worker entry for unit tests.
    fn compact_entry(relative_path: &str, tag: Rating) -> CompactSearchEntry {
        CompactSearchEntry {
            display_label: relative_path.into(),
            relative_path: relative_path.into(),
            tag,
            last_played_at: None,
        }
    }

    fn make_search_job(query: &str, root: &str) -> SearchJob {
        SearchJob {
            request_id: 1,
            source_id: SourceId::new(),
            source_root: std::path::PathBuf::from(root),
            query: query.to_string(),
            filter: TriageFlagFilter::All,
            rating_filter: BTreeSet::new(),
            sort: SampleBrowserSort::ListOrder,
            similar_query: None,
            folder_selection: None,
            folder_negated: None,
            root_mode: crate::app::state::RootFolderFilterMode::AllDescendants,
        }
    }
}
