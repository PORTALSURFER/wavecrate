//! Staged helpers for search cache refresh, scoring, and visible-row construction.

use super::super::telemetry::{
    record_search_worker_score_alloc, record_search_worker_scratch_alloc,
    record_search_worker_similar_lookup_alloc,
};
use super::folders::{filter_accepts_tag, folder_accepts_for_job, folder_accepts_index};
use super::results::sort_visible_by_playback_age;
use super::*;

/// Ensure the worker cache targets the job source, reopening DB/caches on source or stamp changes.
pub(super) fn ensure_search_cache_ready_for_job(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    source_id: &str,
) -> bool {
    let db_path = crate::sample_sources::database_path_for(&job.source_root);
    let db_stamp = DbFileStamp::from_path(&db_path);
    let must_reopen = cache.db.is_none()
        || cache.source_id.as_deref() != Some(source_id)
        || cache.source_root.as_ref() != Some(&job.source_root)
        || cache.db_stamp.as_ref() != db_stamp.as_ref();
    if !must_reopen {
        return true;
    }

    match crate::sample_sources::SourceDatabase::open_read_only(&job.source_root) {
        Ok(db) => {
            cache.db = Some(db);
            cache.entries = None;
            cache.revision = 0;
            cache.source_id = Some(source_id.to_string());
            cache.source_root = Some(job.source_root.clone());
            cache.db_stamp = db_stamp;
            cache.query_score_cache.clear();
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            true
        }
        Err(_) => {
            cache.db = None;
            cache.entries = None;
            cache.revision = 0;
            cache.source_id = Some(source_id.to_string());
            cache.source_root = Some(job.source_root.clone());
            cache.db_stamp = db_stamp;
            cache.query_score_cache.clear();
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            false
        }
    }
}

/// Load compact search entries when DB revision changes or cache is empty.
pub(super) fn ensure_search_entries_loaded_for_job(
    cache: &mut SearchWorkerCache,
    _job: &SearchJob,
) -> bool {
    let Some(db) = cache.db.as_ref() else {
        return false;
    };
    let revision = db.get_revision().unwrap_or(0);
    let must_reload = cache.entries.is_none() || cache.revision != revision;
    if !must_reload {
        return true;
    }

    match db.list_files() {
        Ok(loaded_entries) => {
            let compact_entries: Vec<CompactSearchEntry> = loaded_entries
                .into_iter()
                .map(|entry| {
                    let relative_path = entry.relative_path.to_string_lossy().to_string();
                    let display_label =
                        crate::app::view_model::sample_display_label(&entry.relative_path);
                    CompactSearchEntry {
                        display_label: display_label.into_boxed_str(),
                        relative_path: relative_path.into_boxed_str(),
                        tag: entry.tag,
                        last_played_at: entry.last_played_at,
                    }
                })
                .collect();
            cache.entries = Some(compact_entries);
            cache.revision = revision;
            cache.query_score_cache.clear();
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            true
        }
        Err(_) => {
            cache.entries = None;
            cache.query_score_cache.clear();
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            false
        }
    }
}

/// Resolve fuzzy query scores, reusing cached scores when query/source/revision still match.
pub(super) fn resolve_query_scores_for_job(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    matcher: &SkimMatcherV2,
    queue: &SearchJobQueue,
    generation: u64,
    source_id: &str,
    entries_len: usize,
) -> Option<Arc<[Option<i64>]>> {
    if job.query.is_empty() {
        return Some(Arc::from([]));
    }

    if let Some(scores) =
        try_reuse_cached_query_scores(cache, source_id, cache.revision, &job.query, entries_len)
    {
        return Some(scores);
    }

    let added_score_capacity = cache.prepare_score_scratch(entries_len);
    record_search_worker_score_alloc(
        added_score_capacity.saturating_mul(std::mem::size_of::<Option<i64>>()),
    );
    let Some(entries) = cache.entries.as_ref() else {
        return Some(Arc::from([]));
    };

    for (index, entry) in entries.iter().enumerate() {
        if super::search_job_canceled_for_index(queue, generation, index) {
            return None;
        }
        cache.score_scratch[index] = matcher.fuzzy_match(&entry.display_label, &job.query);
    }
    if super::search_job_canceled(queue, generation) {
        return None;
    }

    let computed_scores: Arc<[Option<i64>]> = Arc::from(cache.score_scratch.as_slice());
    cache.query_score_cache.insert(
        0,
        WorkerQueryScoreCacheEntry {
            source_id: source_id.to_string(),
            revision: cache.revision,
            query: job.query.clone(),
            scores: Arc::clone(&computed_scores),
        },
    );
    cache.query_score_cache.truncate(cache.max_cached_queries);
    Some(computed_scores)
}

/// Reuse and promote a matching query-score cache entry for LRU behavior.
pub(super) fn try_reuse_cached_query_scores(
    cache: &mut SearchWorkerCache,
    source_id: &str,
    revision: u64,
    query: &str,
    entries_len: usize,
) -> Option<Arc<[Option<i64>]>> {
    let index = cache.query_score_cache.iter().position(|cached| {
        cached.source_id == source_id
            && cached.revision == revision
            && cached.query == query
            && cached.scores.len() == entries_len
    })?;
    let cached = cache.query_score_cache.remove(index);
    let scores = Arc::clone(&cached.scores);
    cache.query_score_cache.insert(0, cached);
    Some(scores)
}

/// Return an `All` visible-rows result when no filtering/sorting/scoring work is required.
pub(super) fn build_fast_path_result_if_applicable(
    job: &SearchJob,
    has_query: bool,
    has_folder_filters: bool,
    entries_len: usize,
    partitions: &TriagePartitions,
) -> Option<SearchResult> {
    if has_query
        || has_folder_filters
        || job.filter != TriageFlagFilter::All
        || job.similar_query.is_some()
        || job.sort != SampleBrowserSort::ListOrder
        || !job.rating_filter.is_empty()
    {
        return None;
    }

    record_search_worker_visible_rows(entries_len);
    Some(SearchResult {
        request_id: job.request_id,
        source_id: job.source_id.clone(),
        query: job.query.clone(),
        visible: VisibleRows::All { total: entries_len },
        trash: Arc::clone(&partitions.0),
        neutral: Arc::clone(&partitions.1),
        keep: Arc::clone(&partitions.2),
        scores: Arc::from([]),
    })
}

/// Inputs required to build visible search rows for one search job.
pub(super) struct BuildVisibleRowsParams<'a> {
    pub(super) job: &'a SearchJob,
    pub(super) has_query: bool,
    pub(super) scores: &'a Arc<[Option<i64>]>,
    pub(super) entries_len: usize,
    pub(super) queue: &'a SearchJobQueue,
    pub(super) generation: u64,
    pub(super) source_id: &'a str,
    pub(super) has_folder_filters: bool,
}

/// Build filtered visible indices for query/folder/tag/similarity criteria.
pub(super) fn build_visible_rows_for_job(
    cache: &mut SearchWorkerCache,
    params: BuildVisibleRowsParams<'_>,
) -> Option<Vec<usize>> {
    let BuildVisibleRowsParams {
        job,
        has_query,
        scores,
        entries_len,
        queue,
        generation,
        source_id,
        has_folder_filters,
    } = params;
    let folder_accepts =
        folder_accepts_for_job(cache, job, source_id, cache.revision, has_folder_filters);
    if super::search_job_canceled(queue, generation) {
        return None;
    }

    if let Some(similar) = &job.similar_query {
        return build_visible_rows_for_similar(
            cache,
            job,
            similar,
            folder_accepts.as_ref(),
            entries_len,
            queue,
            generation,
        );
    }

    let scratch_capacity = entries_len.min(1024);
    let added_scratch_capacity = cache.prepare_scored_index_scratch(scratch_capacity);
    record_search_worker_scratch_alloc(
        added_scratch_capacity.saturating_mul(std::mem::size_of::<(usize, i64)>()),
    );
    let entries = cache.entries.as_ref()?;

    let mut visible = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if super::search_job_canceled_for_index(queue, generation, index) {
            return None;
        }
        if !filter_accepts_tag(job.filter, &job.rating_filter, entry.tag)
            || !folder_accepts_index(folder_accepts.as_ref(), index)
        {
            continue;
        }

        if has_query {
            if let Some(score) = scores.get(index).and_then(|score| *score) {
                cache.scored_index_scratch.push((index, score));
            }
        } else {
            visible.push(index);
        }
    }

    if has_query {
        if super::search_job_canceled(queue, generation) {
            return None;
        }
        cache
            .scored_index_scratch
            .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        if super::search_job_canceled(queue, generation) {
            return None;
        }
        visible.clear();
        visible.reserve(cache.scored_index_scratch.len());
        visible.extend(cache.scored_index_scratch.iter().map(|(index, _)| *index));
    }

    sort_visible_indices(entries, &mut visible, job.sort);
    if super::search_job_canceled(queue, generation) {
        return None;
    }
    Some(visible)
}

fn build_visible_rows_for_similar(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    similar: &crate::app::state::SimilarQuery,
    folder_accepts: Option<&Arc<[bool]>>,
    entries_len: usize,
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<Vec<usize>> {
    if job.sort == SampleBrowserSort::Similarity {
        let added_lookup_capacity = cache.prepare_similar_lookup_scratch(entries_len);
        record_search_worker_similar_lookup_alloc(
            added_lookup_capacity.saturating_mul(std::mem::size_of::<Option<f32>>()),
        );
        for (offset, (&index, &score)) in similar
            .indices
            .iter()
            .zip(similar.scores.iter())
            .enumerate()
        {
            if super::search_job_canceled_for_index(queue, generation, offset) {
                return None;
            }
            if index < cache.similar_lookup_scratch.len() {
                cache.similar_lookup_scratch[index] = Some(score);
            }
        }
        if super::search_job_canceled(queue, generation) {
            return None;
        }
    }

    let entries = cache.entries.as_ref()?;
    let mut visible = Vec::new();
    for (offset, index) in similar.indices.iter().copied().enumerate() {
        if super::search_job_canceled_for_index(queue, generation, offset) {
            return None;
        }
        if let Some(entry) = entries.get(index)
            && filter_accepts_tag(job.filter, &job.rating_filter, entry.tag)
            && folder_accepts_index(folder_accepts, index)
        {
            visible.push(index);
        }
    }

    if job.sort == SampleBrowserSort::Similarity {
        visible.sort_by(|a: &usize, b: &usize| {
            let a_score = cache
                .similar_lookup_scratch
                .get(*a)
                .and_then(|score| *score)
                .unwrap_or(f32::NEG_INFINITY);
            let b_score = cache
                .similar_lookup_scratch
                .get(*b)
                .and_then(|score| *score)
                .unwrap_or(f32::NEG_INFINITY);
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.cmp(b))
        });
        if super::search_job_canceled(queue, generation) {
            return None;
        }

        if let Some(anchor) = similar.anchor_index
            && let Some(entry) = entries.get(anchor)
            && filter_accepts_tag(job.filter, &job.rating_filter, entry.tag)
            && folder_accepts_index(folder_accepts, anchor)
        {
            if let Some(pos) = visible.iter().position(|index| *index == anchor) {
                visible.remove(pos);
            }
            visible.insert(0, anchor);
        }
    } else {
        sort_visible_indices(entries, &mut visible, job.sort);
        if super::search_job_canceled(queue, generation) {
            return None;
        }
    }

    Some(visible)
}

/// Sort visible indices according to the active browser sort mode.
pub(super) fn sort_visible_indices(
    entries: &[CompactSearchEntry],
    visible: &mut [usize],
    sort: SampleBrowserSort,
) {
    match sort {
        SampleBrowserSort::PlaybackAgeAsc => sort_visible_by_playback_age(entries, visible, true),
        SampleBrowserSort::PlaybackAgeDesc => sort_visible_by_playback_age(entries, visible, false),
        SampleBrowserSort::ListOrder => visible.sort_unstable(),
        SampleBrowserSort::Similarity => {}
    }
}
