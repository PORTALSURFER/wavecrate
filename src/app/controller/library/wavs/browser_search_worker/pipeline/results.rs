//! Search result shaping helpers for empty states, triage partitions, and sorting.

use super::*;

/// Resolve revision-keyed triage partitions, rebuilding only when revision changes.
pub(super) fn triage_partitions_for_revision(
    cache: &mut SearchWorkerCache,
    source_id: &str,
    revision: u64,
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<TriagePartitions> {
    let entries = match cache.entries.as_ref() {
        Some(entries) => entries,
        None => return Some((Arc::from([]), Arc::from([]), Arc::from([]))),
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
            if super::search_job_canceled_for_index(queue, generation, index) {
                return None;
            }
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
        return Some((
            Arc::clone(&cached.trash),
            Arc::clone(&cached.neutral),
            Arc::clone(&cached.keep),
        ));
    }
    Some((Arc::from([]), Arc::from([]), Arc::from([])))
}

/// Build an empty search result while preserving the incoming request metadata.
pub(super) fn empty_search_result_for(job: &SearchJob) -> SearchResult {
    SearchResult {
        request_id: job.request_id,
        source_id: job.source_id.clone(),
        query: job.query.clone(),
        visible: VisibleRows::List(Vec::new().into()),
        trash: Arc::from([]),
        neutral: Arc::from([]),
        keep: Arc::from([]),
        scores: Arc::from([]),
    }
}

/// Build an empty search result from an owned job (legacy helper).
pub(super) fn empty_search_result(job: SearchJob) -> SearchResult {
    empty_search_result_for(&job)
}

/// Sort visible indices by optional playback-age metadata with stable index tie-breakers.
pub(super) fn sort_visible_by_playback_age(
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
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn triage_partition_rebuild_stops_when_generation_turns_stale() {
        let mut cache = SearchWorkerCache {
            entries: Some(
                (0..(super::super::SEARCH_CANCEL_CHECK_INTERVAL + 1))
                    .map(|index| CompactSearchEntry {
                        display_label: format!("item-{index}").into_boxed_str(),
                        relative_path: format!("item-{index}.wav").into_boxed_str(),
                        tag: if index % 2 == 0 {
                            Rating::KEEP_1
                        } else {
                            Rating::TRASH_1
                        },
                        locked: false,
                        last_played_at: None,
                    })
                    .collect(),
            ),
            ..SearchWorkerCache::default()
        };
        let queue = SearchJobQueue::new();
        queue.send(make_search_job("first"));
        let stale = queue
            .take_blocking()
            .expect("expected queued search job generation");
        queue.send(make_search_job("second"));

        let partitions =
            triage_partitions_for_revision(&mut cache, "source-a", 1, &queue, stale.generation);

        assert!(partitions.is_none());
        assert!(cache.triage_cache.is_none());
    }

    fn make_search_job(query: &str) -> SearchJob {
        SearchJob {
            request_id: 1,
            source_id: SourceId::new(),
            source_root: PathBuf::from("root"),
            query: query.to_string(),
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
