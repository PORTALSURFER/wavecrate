//! Search filtering/scoring pipeline and helper routines.

/// Folder/tag filtering cache helpers and key hashing.
mod folders;
/// Result shaping helpers for triage partitions and playback-age sorting.
mod results;

use self::folders::{filter_accepts_tag, folder_accepts_for_job, folder_accepts_index};
use self::results::{empty_search_result, sort_visible_by_playback_age};
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
            let added_score_capacity = cache.prepare_score_scratch(entries_len);
            record_search_worker_score_alloc(
                added_score_capacity.saturating_mul(std::mem::size_of::<Option<i64>>()),
            );
            let Some(entries) = cache.entries.as_ref() else {
                return Some(empty_search_result(job));
            };
            for (index, entry) in entries.iter().enumerate() {
                if search_job_canceled_for_index(queue, generation, index) {
                    return None;
                }
                cache.score_scratch[index] = matcher.fuzzy_match(&entry.display_label, &job.query);
            }
            if search_job_canceled(queue, generation) {
                return None;
            }
            let computed_scores: Arc<[Option<i64>]> = Arc::from(cache.score_scratch.as_slice());
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
    let mut visible = Vec::new();

    if let Some(similar) = &job.similar_query {
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
                if search_job_canceled_for_index(queue, generation, offset) {
                    return None;
                }
                if index < cache.similar_lookup_scratch.len() {
                    cache.similar_lookup_scratch[index] = Some(score);
                }
            }
            if search_job_canceled(queue, generation) {
                return None;
            }
        }
        let Some(entries) = cache.entries.as_ref() else {
            return Some(empty_search_result(job));
        };
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
                visible.sort_by(|a: &usize, b: &usize| {
                    let a_score = cache
                        .similar_lookup_scratch
                        .get(*a)
                        .and_then(|s| *s)
                        .unwrap_or(f32::NEG_INFINITY);
                    let b_score = cache
                        .similar_lookup_scratch
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
        let scratch_capacity = entries_len.min(1024);
        let added_scratch_capacity = cache.prepare_scored_index_scratch(scratch_capacity);
        record_search_worker_scratch_alloc(
            added_scratch_capacity.saturating_mul(std::mem::size_of::<(usize, i64)>()),
        );
        let Some(entries) = cache.entries.as_ref() else {
            return Some(empty_search_result(job));
        };
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
                    cache.scored_index_scratch.push((index, score));
                }
            } else {
                visible.push(index);
            }
        }

        if has_query {
            if search_job_canceled(queue, generation) {
                return None;
            }
            cache
                .scored_index_scratch
                .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            if search_job_canceled(queue, generation) {
                return None;
            }
            visible.clear();
            visible.reserve(cache.scored_index_scratch.len());
            visible.extend(cache.scored_index_scratch.iter().map(|(index, _)| *index));
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

pub(super) fn triage_partitions_for_revision(
    cache: &mut SearchWorkerCache,
    source_id: &str,
    revision: u64,
) -> TriagePartitions {
    results::triage_partitions_for_revision(cache, source_id, revision)
}

pub(super) fn folder_filter_hash_for_job(job: &SearchJob) -> u64 {
    folders::folder_filter_hash_for_job(job)
}
