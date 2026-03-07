//! Search filtering/scoring pipeline and helper routines.

/// Folder/tag filtering cache helpers and key hashing.
mod folders;
/// Result shaping helpers for empty states, triage partitions, and sorting.
mod results;
/// Staged cache/score/visible-row helpers used by `process_search_job`.
mod stages;
#[cfg(test)]
/// Stage-specific search pipeline tests.
mod stages_tests;

use self::results::empty_search_result_for;
use self::stages::{
    BuildVisibleRowsParams, build_fast_path_result_if_applicable, build_visible_rows_for_job,
    ensure_search_cache_ready_for_job, ensure_search_entries_loaded_for_job,
    resolve_query_scores_for_job,
};
use super::cache::*;
use super::queue::SearchJobQueue;
use super::telemetry::{record_search_job_cancel, record_search_worker_visible_rows};
use super::*;

/// Run one queued search request through cache-refresh, scoring, and result shaping stages.
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

    let source_id = job.source_id.as_str().to_string();
    if !ensure_search_cache_ready_for_job(cache, &job, &source_id) {
        return Some(empty_search_result_for(&job));
    }
    if !ensure_search_entries_loaded_for_job(cache, &job, queue, generation) {
        return Some(empty_search_result_for(&job));
    }

    let has_query = !job.query.is_empty();
    let entries_len = cache.entries.as_ref().map(Vec::len).unwrap_or(0);
    let has_folder_filters = crate::app::controller::library::source_folders::folder_filters_active(
        job.folder_selection.as_ref(),
        job.folder_negated.as_ref(),
        job.root_mode,
    );
    let scores = resolve_query_scores_for_job(
        cache,
        &job,
        matcher,
        queue,
        generation,
        &source_id,
        entries_len,
    )?;

    if search_job_canceled(queue, generation) {
        return None;
    }
    let partitions =
        triage_partitions_for_revision(cache, &source_id, cache.revision, queue, generation)?;
    if search_job_canceled(queue, generation) {
        return None;
    }

    if let Some(result) = build_fast_path_result_if_applicable(
        &job,
        has_query,
        has_folder_filters,
        entries_len,
        &partitions,
    ) {
        return Some(result);
    }

    let visible = build_visible_rows_for_job(
        cache,
        BuildVisibleRowsParams {
            job: &job,
            has_query,
            scores: &scores,
            entries_len,
            queue,
            generation,
            source_id: &source_id,
            has_folder_filters,
        },
    )?;

    record_search_worker_visible_rows(visible.len());
    Some(SearchResult {
        request_id: job.request_id,
        source_id: job.source_id,
        query: job.query,
        visible: VisibleRows::List(visible.into()),
        trash: Arc::clone(&partitions.0),
        neutral: Arc::clone(&partitions.1),
        keep: Arc::clone(&partitions.2),
        scores: if has_query { scores } else { Arc::from([]) },
    })
}

const SEARCH_CANCEL_CHECK_INTERVAL: usize = 16;

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

/// Return triage partitions for the current revision, reusing cached partitions when valid.
pub(super) fn triage_partitions_for_revision(
    cache: &mut SearchWorkerCache,
    source_id: &str,
    revision: u64,
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<TriagePartitions> {
    results::triage_partitions_for_revision(cache, source_id, revision, queue, generation)
}

/// Hash folder-filter inputs used to key per-query folder acceptance caches.
pub(super) fn folder_filter_hash_for_job(job: &SearchJob) -> u64 {
    folders::folder_filter_hash_for_job(job)
}

#[cfg(test)]
/// Search pipeline cancellation/hash regression tests.
mod tests {
    use super::*;
    use crate::sample_sources::SourceId;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    /// Cancellation checks only run on the configured periodic interval.
    fn search_job_canceled_for_index_checks_every_interval() {
        let queue = SearchJobQueue::new();
        queue.send(make_search_job("first", "root"));
        let first = match queue.take_blocking() {
            Some(job) => job,
            None => panic!("expected queued search job"),
        };
        assert!(!search_job_canceled_for_index(
            &queue,
            first.generation,
            SEARCH_CANCEL_CHECK_INTERVAL - 1
        ));

        queue.send(make_search_job("second", "root"));
        assert!(search_job_canceled_for_index(
            &queue,
            first.generation,
            SEARCH_CANCEL_CHECK_INTERVAL
        ));
    }

    #[test]
    /// Folder filter hashing must incorporate root mode for cache correctness.
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

    /// Build a minimal search job fixture for cancellation/hash tests.
    fn make_search_job(query: &str, root: &str) -> SearchJob {
        SearchJob {
            request_id: 1,
            source_id: SourceId::new(),
            source_root: PathBuf::from(root),
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
