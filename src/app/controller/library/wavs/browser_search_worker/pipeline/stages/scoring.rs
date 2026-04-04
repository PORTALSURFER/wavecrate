//! Search-worker fuzzy-score cache and scoring helpers.

use super::super::super::telemetry::record_search_worker_score_alloc;
use super::super::*;

/// Resolve fuzzy query scores, reusing cached scores when query/source/path set still match.
pub(in super::super) fn resolve_query_scores_for_job(
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

    let scope = WorkerQueryScoreCacheScope {
        source_id: source_id.to_string(),
        path_fingerprint: cache.path_fingerprint,
    };
    if let Some(cached) = promote_exact_query_score_cache_entry(
        &mut cache.query_score_cache,
        &scope,
        &job.query,
        entries_len,
    ) {
        return Some(Arc::clone(&cached.scores));
    }
    let prefix_cache = reusable_prefix_query_score_cache_entry(
        &cache.query_score_cache,
        &scope,
        &job.query,
        entries_len,
    );

    let added_score_capacity = cache.prepare_score_scratch(entries_len);
    record_search_worker_score_alloc(
        added_score_capacity.saturating_mul(std::mem::size_of::<Option<i64>>()),
    );
    let Some(entries) = cache.entries.as_ref() else {
        return Some(Arc::from([]));
    };
    let candidate_indices = prefix_cache
        .as_ref()
        .map(|cached| cached.matched_indices.as_ref());
    let matched_indices = score_query_candidates(
        &mut cache.score_scratch,
        candidate_indices,
        entries_len,
        |index, offset| {
            if search_job_canceled_for_index(queue, generation, offset) {
                return ScoreCandidateResult::Cancel;
            }
            let Some(entry) = entries.get(index) else {
                return ScoreCandidateResult::Continue(None);
            };
            ScoreCandidateResult::Continue(matcher.fuzzy_match(&entry.display_label, &job.query))
        },
    )?;
    if search_job_canceled(queue, generation) {
        return None;
    }

    let computed_scores: Arc<[Option<i64>]> = Arc::from(cache.score_scratch.as_slice());
    store_query_score_cache_entry(
        &mut cache.query_score_cache,
        cache.max_cached_queries,
        scope,
        job.query.clone(),
        Arc::clone(&computed_scores),
        matched_indices,
    );
    Some(computed_scores)
}

/// Reuse and promote a matching query-score cache entry for LRU behavior.
pub(in super::super) fn try_reuse_cached_query_scores(
    cache: &mut SearchWorkerCache,
    source_id: &str,
    path_fingerprint: u64,
    query: &str,
    entries_len: usize,
) -> Option<Arc<[Option<i64>]>> {
    let scope = WorkerQueryScoreCacheScope {
        source_id: source_id.to_string(),
        path_fingerprint,
    };
    promote_exact_query_score_cache_entry(&mut cache.query_score_cache, &scope, query, entries_len)
        .map(|cached| Arc::clone(&cached.scores))
}

/// Return the longest reusable prefix-query score cache entry.
pub(in super::super) fn reusable_prefix_query_scores(
    cache: &SearchWorkerCache,
    source_id: &str,
    path_fingerprint: u64,
    query: &str,
    entries_len: usize,
) -> Option<WorkerQueryScoreCacheEntry> {
    let scope = WorkerQueryScoreCacheScope {
        source_id: source_id.to_string(),
        path_fingerprint,
    };
    reusable_prefix_query_score_cache_entry(&cache.query_score_cache, &scope, query, entries_len)
}
