//! Search-worker visible-row fast paths and filtered row builders.

use super::super::super::telemetry::{
    record_search_worker_scratch_alloc, record_search_worker_similar_lookup_alloc,
    record_search_worker_visible_rows,
};
use super::super::folders::{filter_accepts_tag, folder_accepts_for_job, folder_accepts_index};
use super::super::results::sort_visible_by_playback_age;
use super::super::*;
use std::path::Path;

/// Return an `All` visible-rows result when no filtering/sorting/scoring work is required.
pub(in super::super) fn build_fast_path_result_if_applicable(
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
        || job.duplicate_cleanup.is_some()
        || job.sort != SampleBrowserSort::ListOrder
        || !job.rating_filter.is_empty()
        || job.marked_only
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
pub(in super::super) struct BuildVisibleRowsParams<'a> {
    pub(in super::super) job: &'a SearchJob,
    pub(in super::super) has_query: bool,
    pub(in super::super) scores: &'a Arc<[Option<i64>]>,
    pub(in super::super) entries_len: usize,
    pub(in super::super) queue: &'a SearchJobQueue,
    pub(in super::super) generation: u64,
    pub(in super::super) source_id: &'a str,
    pub(in super::super) has_folder_filters: bool,
}

/// Build filtered visible indices for query/folder/tag/similarity criteria.
pub(in super::super) fn build_visible_rows_for_job(
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
    let folder_accepts = folder_accepts_for_job(
        cache,
        job,
        source_id,
        cache.revision,
        has_folder_filters,
        queue,
        generation,
    )?;
    if search_job_canceled(queue, generation) {
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

    if let Some(cleanup) = &job.duplicate_cleanup {
        return Some(
            cleanup
                .indices
                .iter()
                .copied()
                .filter(|index| *index < entries_len)
                .collect(),
        );
    }

    let query_needs_score_sort = has_query;
    if query_needs_score_sort {
        let scratch_capacity = entries_len.min(1024);
        let added_scratch_capacity = cache.prepare_scored_index_scratch(scratch_capacity);
        record_search_worker_scratch_alloc(
            added_scratch_capacity.saturating_mul(std::mem::size_of::<(usize, i64)>()),
        );
    } else {
        cache.scored_index_scratch.clear();
    }
    let entries = cache.entries.as_ref()?;

    let mut visible = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if search_job_canceled_for_index(queue, generation, index) {
            return None;
        }
        let marked = job
            .marked_paths
            .contains(Path::new(entry.relative_path.as_ref()));
        if !filter_accepts_tag(
            job.filter,
            &job.rating_filter,
            job.marked_only,
            marked,
            entry.tag,
            entry.locked,
        )
            || !folder_accepts_index(folder_accepts.as_ref(), index)
        {
            continue;
        }

        if has_query {
            if let Some(score) = scores.get(index).and_then(|score| *score) {
                if query_needs_score_sort {
                    cache.scored_index_scratch.push((index, score));
                } else {
                    visible.push(index);
                }
            }
        } else {
            visible.push(index);
        }
    }

    if query_needs_score_sort {
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

    if job.sort != SampleBrowserSort::ListOrder {
        sort_visible_indices(entries, &mut visible, job.sort);
        if search_job_canceled(queue, generation) {
            return None;
        }
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

    let entries = cache.entries.as_ref()?;
    let mut visible = Vec::new();
    for (offset, index) in similar.indices.iter().copied().enumerate() {
        if search_job_canceled_for_index(queue, generation, offset) {
            return None;
        }
        if let Some(entry) = entries.get(index)
            && {
                let marked = job
                    .marked_paths
                    .contains(Path::new(entry.relative_path.as_ref()));
                filter_accepts_tag(
                    job.filter,
                    &job.rating_filter,
                    job.marked_only,
                    marked,
                    entry.tag,
                    entry.locked,
                )
            }
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
        if search_job_canceled(queue, generation) {
            return None;
        }

        if let Some(anchor) = similar.anchor_index
            && let Some(entry) = entries.get(anchor)
            && filter_accepts_tag(
                job.filter,
                &job.rating_filter,
                job.marked_only,
                job.marked_paths
                    .contains(Path::new(entry.relative_path.as_ref())),
                entry.tag,
                entry.locked,
            )
            && folder_accepts_index(folder_accepts, anchor)
        {
            if let Some(pos) = visible.iter().position(|index| *index == anchor) {
                visible.remove(pos);
            }
            visible.insert(0, anchor);
        }
    } else {
        sort_visible_indices(entries, &mut visible, job.sort);
        if search_job_canceled(queue, generation) {
            return None;
        }
    }

    Some(visible)
}

/// Sort visible indices according to the active browser sort mode.
pub(in super::super) fn sort_visible_indices(
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
