//! Retained filter-stage caches for worker-side browser search composition.

use super::super::folders::{
    filter_accepts_tag, folder_accepts_for_job, folder_accepts_index,
};
use super::super::*;
use std::hash::{Hash, Hasher};
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
    if let Some(index) = cache.filter_stage_cache.iter().position(|cached| {
        cached.source_id == source_id
            && cached.revision == cache.revision
            && cached.filter_hash == filter_hash
            && cached.accepts.len() == entries_len
    }) {
        let cached = cache.filter_stage_cache.remove(index);
        cache.filter_stage_cache.insert(0, cached);
        return Some(cache.filter_stage_cache.first().map(|cached| WorkerFilteredStage {
            accepts: Arc::clone(&cached.accepts),
            rows: Arc::clone(&cached.rows),
        }));
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
        let accepted = filter_accepts_tag(
            job.filter,
            &job.rating_filter,
            &job.playback_age_filter,
            job.marked_only,
            marked,
            entry.tag,
            entry.locked,
            entry.last_played_at,
            job.playback_age_now_unix_secs,
        ) && folder_accepts_index(folder_accepts.as_ref(), index);
        accepts.push(accepted);
        if accepted {
            rows.push(index);
        }
    }
    if super::super::search_job_canceled(queue, generation) {
        return None;
    }

    cache.filter_stage_cache.insert(
        0,
        WorkerFilterStageCacheEntry {
            source_id: source_id.to_string(),
            revision: cache.revision,
            filter_hash,
            accepts: Arc::from(accepts),
            rows: Arc::from(rows),
        },
    );
    cache
        .filter_stage_cache
        .truncate(cache.max_cached_filter_stages);
    Some(cache.filter_stage_cache.first().map(|cached| WorkerFilteredStage {
        accepts: Arc::clone(&cached.accepts),
        rows: Arc::clone(&cached.rows),
    }))
}

fn filter_stage_required(job: &SearchJob, has_folder_filters: bool) -> bool {
    has_folder_filters
        || job.filter != TriageFlagFilter::All
        || !job.rating_filter.is_empty()
        || !job.playback_age_filter.is_empty()
        || job.marked_only
}

fn filter_stage_hash(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    has_folder_filters: bool,
) -> u64 {
    hash_value(&(
        filter_key(job.filter),
        hash_value(&job.rating_filter),
        hash_value(&job.playback_age_filter),
        playback_age_filter_cache_token(cache, &job.playback_age_filter, job.playback_age_now_unix_secs),
        job.marked_only,
        job.marked_only.then_some(hash_value(&job.marked_paths)),
        has_folder_filters.then_some(super::super::folder_filter_hash_for_job(job)),
    ))
}

fn filter_key(filter: TriageFlagFilter) -> u8 {
    match filter {
        TriageFlagFilter::All => 0,
        TriageFlagFilter::Keep => 1,
        TriageFlagFilter::Trash => 2,
        TriageFlagFilter::Untagged => 3,
    }
}

fn playback_age_filter_cache_token(
    cache: &mut SearchWorkerCache,
    filters: &std::collections::BTreeSet<crate::app::state::PlaybackAgeFilterChip>,
    now_unix_secs: i64,
) -> Option<i64> {
    if filters.is_empty() {
        return None;
    }
    let filter_hash = hash_value(filters);
    if let Some(cached) = cache
        .playback_age_token_caches
        .iter()
        .copied()
        .find(|cached| cached.revision == cache.revision && cached.filter_hash == filter_hash)
        && cached.token.is_none_or(|token| now_unix_secs < token)
    {
        return cached.token;
    }

    let token = cache
        .entries
        .as_ref()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            crate::app::state::next_playback_age_filter_change_unix_secs(
                filters,
                entry.last_played_at,
                now_unix_secs,
            )
        })
        .min();
    let cache_entry = WorkerPlaybackAgeTokenCache {
        revision: cache.revision,
        filter_hash,
        token,
    };
    if let Some(index) = cache.playback_age_token_caches.iter().position(|cached| {
        cached.revision == cache.revision && cached.filter_hash == filter_hash
    }) {
        cache.playback_age_token_caches[index] = cache_entry;
    } else {
        cache.playback_age_token_caches.push(cache_entry);
    }
    token
}

fn hash_value<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_sources::SourceId;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn playback_age_filter_token_reuses_cached_boundary() {
        let mut cache = SearchWorkerCache {
            revision: 7,
            entries: Some(vec![CompactSearchEntry {
                display_label: "aging".into(),
                relative_path: "aging.wav".into(),
                tag: Rating::NEUTRAL,
                locked: false,
                last_played_at: Some(100),
            }]),
            ..SearchWorkerCache::default()
        };
        let filters = BTreeSet::from([crate::app::state::PlaybackAgeFilterChip::OlderThanWeek]);
        let before = playback_age_filter_cache_token(&mut cache, &filters, 100 + (7 * 24 * 60 * 60) - 2);
        let again = playback_age_filter_cache_token(&mut cache, &filters, 100 + (7 * 24 * 60 * 60) - 1);

        assert_eq!(before, Some(100 + (7 * 24 * 60 * 60)));
        assert_eq!(again, before);
        assert_eq!(cache.playback_age_token_caches.len(), 1);
    }

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
                },
                CompactSearchEntry {
                    display_label: "trash".into(),
                    relative_path: "trash.wav".into(),
                    tag: Rating::TRASH_1,
                    locked: false,
                    last_played_at: None,
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

        let first = filtered_stage_for_job(
            &mut cache,
            &job,
            "source-a",
            2,
            false,
            &queue,
            generation,
        )
        .expect("filter stage build")
        .expect("filter stage");
        let second = filtered_stage_for_job(
            &mut cache,
            &job,
            "source-a",
            2,
            false,
            &queue,
            generation,
        )
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
            marked_paths: BTreeSet::new(),
            sort: SampleBrowserSort::ListOrder,
            similar_query: None,
            duplicate_cleanup: None,
            folder_selection: None,
            folder_negated: None,
            file_scope_mode: crate::app::state::FolderFileScopeMode::AllDescendants,
            playback_age_now_unix_secs: 0,
        }
    }
}
