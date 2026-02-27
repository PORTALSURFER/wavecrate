//! Search filtering/scoring pipeline and helper routines.

use super::cache::*;
use super::queue::SearchJobQueue;
use super::telemetry::{
    record_search_job_cancel, record_search_worker_score_alloc, record_search_worker_scratch_alloc,
    record_search_worker_similar_lookup_alloc, record_search_worker_visible_rows,
};
use super::*;

pub(super) fn process_search_job(
    job: SearchJob,
    matcher: &SkimMatcherV2,
    cache: &mut SearchWorkerCache,
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<SearchResult> {
    if search_job_canceled(queue, generation) {
        return None;
    }
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
                return Some(empty_search_result(job));
            }
        }
    }

    let db = match cache.db.as_ref() {
        Some(db) => db,
        None => return Some(empty_search_result(job)),
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
                return Some(empty_search_result(job));
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
    let mut scores: Arc<[Option<i64>]> = Arc::from([]);

    if has_query {
        let Some(entries) = cache.entries.as_ref() else {
            return Some(empty_search_result(job));
        };
        if let Some(index) = cache.query_score_cache.iter().position(|cached| {
            cached.source_id == job_source_id_str
                && cached.revision == cache.revision
                && cached.query == job.query
                && cached.scores.len() == entries_len
        }) {
            let cached = cache.query_score_cache.remove(index);
            scores = Arc::clone(&cached.scores);
            cache.query_score_cache.insert(0, cached);
        } else {
            let mut computed_scores = vec![None; entries_len];
            record_search_worker_score_alloc(
                entries_len.saturating_mul(std::mem::size_of::<Option<i64>>()),
            );
            for (index, entry) in entries.iter().enumerate() {
                if search_job_canceled_for_index(queue, generation, index) {
                    return None;
                }
                computed_scores[index] = matcher.fuzzy_match(&entry.display_label, &job.query);
            }
            if search_job_canceled(queue, generation) {
                return None;
            }
            let computed_scores: Arc<[Option<i64>]> = computed_scores.into();
            cache.query_score_cache.insert(
                0,
                WorkerQueryScoreCacheEntry {
                    source_id: job_source_id_str.clone(),
                    revision: cache.revision,
                    query: job.query.clone(),
                    scores: Arc::clone(&computed_scores),
                },
            );
            cache.query_score_cache.truncate(cache.max_cached_queries);
            scores = computed_scores;
        }
    }

    if search_job_canceled(queue, generation) {
        return None;
    }
    let (trash, neutral, keep) =
        triage_partitions_for_revision(cache, &job_source_id_str, cache.revision);
    if search_job_canceled(queue, generation) {
        return None;
    }
    if !has_query
        && !has_folder_filters
        && job.filter == TriageFlagFilter::All
        && job.similar_query.is_none()
        && job.sort == SampleBrowserSort::ListOrder
        && job.rating_filter.is_empty()
    {
        record_search_worker_visible_rows(entries_len);
        return Some(SearchResult {
            request_id: job.request_id,
            source_id: job.source_id,
            query: job.query,
            visible: VisibleRows::All { total: entries_len },
            trash: Arc::clone(&trash),
            neutral: Arc::clone(&neutral),
            keep: Arc::clone(&keep),
            scores: Arc::from([]),
        });
    }

    let folder_accepts = folder_accepts_for_job(
        cache,
        &job,
        &job_source_id_str,
        cache.revision,
        has_folder_filters,
    );
    if search_job_canceled(queue, generation) {
        return None;
    }
    let Some(entries) = cache.entries.as_ref() else {
        return Some(empty_search_result(job));
    };
    let mut visible = Vec::new();

    if let Some(similar) = &job.similar_query {
        for (offset, index) in similar.indices.iter().copied().enumerate() {
            if search_job_canceled_for_index(queue, generation, offset) {
                return None;
            }
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
                record_search_worker_similar_lookup_alloc(
                    entries
                        .len()
                        .saturating_mul(std::mem::size_of::<Option<f32>>()),
                );
                for (offset, (&index, &score)) in similar
                    .indices
                    .iter()
                    .zip(similar.scores.iter())
                    .enumerate()
                {
                    if search_job_canceled_for_index(queue, generation, offset) {
                        return None;
                    }
                    if index < score_lookup.len() {
                        score_lookup[index] = Some(score);
                    }
                }
                if search_job_canceled(queue, generation) {
                    return None;
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
                if search_job_canceled(queue, generation) {
                    return None;
                }

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
                if search_job_canceled(queue, generation) {
                    return None;
                }
                sort_visible_by_playback_age(entries, &mut visible, true);
            }
            SampleBrowserSort::PlaybackAgeDesc => {
                if search_job_canceled(queue, generation) {
                    return None;
                }
                sort_visible_by_playback_age(entries, &mut visible, false);
            }
            SampleBrowserSort::ListOrder => {
                if search_job_canceled(queue, generation) {
                    return None;
                }
                visible.sort_unstable();
            }
        }
    } else {
        let scratch_capacity = entries.len().min(1024);
        let mut scratch = Vec::with_capacity(scratch_capacity);
        record_search_worker_scratch_alloc(
            scratch_capacity.saturating_mul(std::mem::size_of::<(usize, i64)>()),
        );
        for (index, entry) in entries.iter().enumerate() {
            if search_job_canceled_for_index(queue, generation, index) {
                return None;
            }
            if !filter_accepts_tag(job.filter, &job.rating_filter, entry.tag)
                || !folder_accepts_index(folder_accepts.as_ref(), index)
            {
                continue;
            }
            if has_query {
                if let Some(score) = scores.get(index).and_then(|score| *score) {
                    scratch.push((index, score));
                }
            } else {
                visible.push(index);
            }
        }

        if has_query {
            if search_job_canceled(queue, generation) {
                return None;
            }
            scratch.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            if search_job_canceled(queue, generation) {
                return None;
            }
            visible = scratch.into_iter().map(|(index, _)| index).collect();
        }
        match job.sort {
            SampleBrowserSort::PlaybackAgeAsc => {
                if search_job_canceled(queue, generation) {
                    return None;
                }
                sort_visible_by_playback_age(entries, &mut visible, true);
            }
            SampleBrowserSort::PlaybackAgeDesc => {
                if search_job_canceled(queue, generation) {
                    return None;
                }
                sort_visible_by_playback_age(entries, &mut visible, false);
            }
            _ => {}
        }
    }

    record_search_worker_visible_rows(visible.len());
    Some(SearchResult {
        request_id: job.request_id,
        source_id: job.source_id,
        query: job.query,
        visible: VisibleRows::List(visible.into()),
        trash: Arc::clone(&trash),
        neutral: Arc::clone(&neutral),
        keep: Arc::clone(&keep),
        scores: if has_query { scores } else { Arc::from([]) },
    })
}

const SEARCH_CANCEL_CHECK_INTERVAL: usize = 64;

fn search_job_canceled(queue: &SearchJobQueue, generation: u64) -> bool {
    let canceled = !queue.is_generation_current(generation);
    if canceled {
        record_search_job_cancel();
    }
    canceled
}

fn search_job_canceled_for_index(queue: &SearchJobQueue, generation: u64, index: usize) -> bool {
    index.is_multiple_of(SEARCH_CANCEL_CHECK_INTERVAL) && search_job_canceled(queue, generation)
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
pub(super) fn triage_partitions_for_revision(
    cache: &mut SearchWorkerCache,
    source_id: &str,
    revision: u64,
) -> TriagePartitions {
    let entries = match cache.entries.as_ref() {
        Some(entries) => entries,
        None => return (Arc::from([]), Arc::from([]), Arc::from([])),
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
            trash: trash.into(),
            neutral: neutral.into(),
            keep: keep.into(),
        });
    }
    if let Some(cached) = cache.triage_cache.as_ref() {
        return (
            Arc::clone(&cached.trash),
            Arc::clone(&cached.neutral),
            Arc::clone(&cached.keep),
        );
    }
    (Arc::from([]), Arc::from([]), Arc::from([]))
}

/// Hash a folder-filter payload into a stable worker cache key.
pub(super) fn folder_filter_hash_for_job(job: &SearchJob) -> u64 {
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
        trash: Arc::from([]),
        neutral: Arc::from([]),
        keep: Arc::from([]),
        scores: Arc::from([]),
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
