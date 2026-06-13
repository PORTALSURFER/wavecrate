//! Retained filter-stage caches for worker-side browser search composition.

mod acceptance;
mod cache_key;
mod playback_age_token;
mod storage;

use self::acceptance::entry_accepted_by_job;
use self::cache_key::{filter_stage_hash, filter_stage_required};
use self::storage::{reuse_cached_stage, store_filter_stage};
use super::super::folders::{folder_accepts_for_job, folder_accepts_index};
use super::super::*;
use std::path::Path;

/// One retained worker filter stage aligned to source-order browser rows.
#[derive(Clone)]
pub(in super::super) struct WorkerFilteredStage {
    /// Combined filter acceptance aligned to absolute entry indices.
    pub(in super::super) accepts: Arc<[bool]>,
    /// Accepted entry indices in source order.
    pub(in super::super) rows: Arc<[usize]>,
}

/// Build or reuse the retained filter stage for the current job.
pub(in super::super) fn filtered_stage_for_job(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    source_id: &str,
    entries_len: usize,
    has_folder_filters: bool,
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<Option<WorkerFilteredStage>> {
    if !filter_stage_required(job, has_folder_filters) {
        return Some(None);
    }
    let folder_accepts = folder_accepts_for_job(
        cache,
        job,
        source_id,
        cache.revision,
        has_folder_filters,
        queue,
        generation,
    )?;
    if super::super::search_job_canceled(queue, generation) {
        return None;
    }

    let filter_hash = filter_stage_hash(cache, job, has_folder_filters);
    if let Some(stage) = reuse_cached_stage(cache, source_id, entries_len, filter_hash) {
        return Some(Some(stage));
    }

    let Some(entries) = cache.entries.as_ref() else {
        return Some(Some(WorkerFilteredStage {
            accepts: Arc::from([]),
            rows: Arc::from([]),
        }));
    };
    let mut accepts = Vec::with_capacity(entries_len);
    let mut rows = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if super::super::search_job_canceled_for_index(queue, generation, index) {
            return None;
        }
        let marked = job
            .marked_paths
            .contains(Path::new(entry.relative_path.as_ref()));
        let relative_path = Path::new(entry.relative_path.as_ref());
        let bpm = job.sidebar_bpm_values.get(relative_path).copied().flatten();
        let accepted = entry_accepted_by_job(job, entry, relative_path, marked, bpm)
            && folder_accepts_index(folder_accepts.as_ref(), index);
        accepts.push(accepted);
        if accepted {
            rows.push(index);
        }
    }
    if super::super::search_job_canceled(queue, generation) {
        return None;
    }

    Some(store_filter_stage(
        cache,
        source_id,
        filter_hash,
        accepts,
        rows,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_sources::SourceId;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn filtered_stage_cache_reuses_matching_filter_shape() {
        let mut cache = SearchWorkerCache {
            revision: 3,
            entries: Some(vec![
                CompactSearchEntry {
                    display_label: "keep".into(),
                    relative_path: "keep.wav".into(),
                    tag: Rating::KEEP_1,
                    locked: false,
                    last_played_at: None,
                    tag_named: false,
                },
                CompactSearchEntry {
                    display_label: "trash".into(),
                    relative_path: "trash.wav".into(),
                    tag: Rating::TRASH_1,
                    locked: false,
                    last_played_at: None,
                    tag_named: false,
                },
            ]),
            ..SearchWorkerCache::default()
        };
        let queue = SearchJobQueue::new();
        queue.send(make_search_job());
        let generation = queue
            .take_blocking()
            .expect("expected queued job generation")
            .generation;
        let job = SearchJob {
            filter: TriageFlagFilter::Keep,
            ..make_search_job()
        };

        let first =
            filtered_stage_for_job(&mut cache, &job, "source-a", 2, false, &queue, generation)
                .expect("filter stage build")
                .expect("filter stage");
        let second =
            filtered_stage_for_job(&mut cache, &job, "source-a", 2, false, &queue, generation)
                .expect("filter stage reuse")
                .expect("filter stage");

        assert_eq!(first.rows.as_ref(), &[0]);
        assert_eq!(cache.filter_stage_cache.len(), 1);
        assert!(Arc::ptr_eq(&first.rows, &second.rows));
        assert!(Arc::ptr_eq(&first.accepts, &second.accepts));
    }

    fn make_search_job() -> SearchJob {
        SearchJob {
            request_id: 1,
            source_id: SourceId::new(),
            source_root: PathBuf::from("root"),
            query: String::new(),
            filter: TriageFlagFilter::All,
            rating_filter: BTreeSet::new(),
            playback_age_filter: BTreeSet::new(),
            marked_only: false,
            tag_named_filter: crate::app::state::TagNamedFilter::All,
            sidebar_filters: Default::default(),
            sidebar_bpm_values: Default::default(),
            marked_paths: BTreeSet::new(),
            sort: SampleBrowserSort::ListOrder,
            similar_query: None,
            duplicate_cleanup: None,
            folder_selection: None,
            folder_negated: None,
            file_scope_mode: crate::app::state::FolderFileScopeMode::AllDescendants,
            metadata_delta_paths: Vec::new(),
            playback_age_now_unix_secs: 0,
        }
    }
}
