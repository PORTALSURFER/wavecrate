//! Folder and tag filtering helpers for search worker pipeline execution.

use super::*;
use std::path::Path;

/// Return whether a tag passes the active triage + rating filter settings.
pub(super) fn filter_accepts_tag(
    filter: TriageFlagFilter,
    rating_filter: &std::collections::BTreeSet<i8>,
    marked_only: bool,
    marked: bool,
    tag: Rating,
    locked: bool,
) -> bool {
    let triage_ok = match filter {
        TriageFlagFilter::All => true,
        TriageFlagFilter::Keep => tag.is_keep(),
        TriageFlagFilter::Trash => tag.is_trash(),
        TriageFlagFilter::Untagged => tag.is_neutral(),
    };
    let rating_level = browser_rating_filter_level(tag, locked);
    let rating_ok = rating_filter.is_empty() || rating_filter.contains(&rating_level);
    let marked_ok = !marked_only || marked;
    triage_ok && rating_ok && marked_ok
}

/// Return the effective browser rating-filter level for one worker entry.
fn browser_rating_filter_level(tag: Rating, locked: bool) -> i8 {
    if locked && tag.is_keep() {
        4
    } else {
        tag.val()
    }
}

/// Return whether a row index passes the cached folder-filter acceptance map.
pub(super) fn folder_accepts_index(folder_accepts: Option<&Arc<[bool]>>, index: usize) -> bool {
    folder_accepts
        .map(|accepts| accepts.get(index).copied().unwrap_or(false))
        .unwrap_or(true)
}

/// Resolve the cached folder-filter acceptance map for the current job.
pub(super) fn folder_accepts_for_job(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    source_id: &str,
    revision: u64,
    has_folder_filters: bool,
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<Option<Arc<[bool]>>> {
    if !has_folder_filters {
        return Some(None);
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
        return Some(
            cache
                .folder_accept_cache
                .first()
                .map(|cached| Arc::clone(&cached.accepts)),
        );
    }

    let accepts = match cache.entries.as_ref() {
        Some(entries) => build_folder_accepts(entries, job, queue, generation)?,
        None => Vec::new(),
    };
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
    Some(
        cache
            .folder_accept_cache
            .first()
            .map(|cached| Arc::clone(&cached.accepts)),
    )
}

/// Build folder-filter acceptance values for all entries in source order.
fn build_folder_accepts(
    entries: &[CompactSearchEntry],
    job: &SearchJob,
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<Vec<bool>> {
    for index in 0..entries.len() {
        if super::search_job_canceled_for_index(queue, generation, index) {
            return None;
        }
    }
    Some(
        crate::app::controller::library::source_folders::build_folder_filter_acceptance_map(
            entries
                .iter()
                .map(|entry| Some(Path::new(entry.relative_path.as_ref()))),
            job.folder_selection.as_ref(),
            job.folder_negated.as_ref(),
            job.file_scope_mode,
        ),
    )
}

/// Hash a folder-filter payload into a stable worker cache key.
pub(super) fn folder_filter_hash_for_job(job: &SearchJob) -> u64 {
    crate::app::controller::library::source_folders::folder_filter_fingerprint(
        job.folder_selection.as_ref(),
        job.folder_negated.as_ref(),
        job.file_scope_mode,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_sources::SourceId;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn locked_keep_filter_accepts_only_locked_keep_rows() {
        let rating_filter = BTreeSet::from([4]);

        assert!(filter_accepts_tag(
            TriageFlagFilter::All,
            &rating_filter,
            false,
            true,
            Rating::KEEP_3,
            true,
        ));
        assert!(!filter_accepts_tag(
            TriageFlagFilter::All,
            &rating_filter,
            false,
            false,
            Rating::KEEP_3,
            false,
        ));
        assert!(!filter_accepts_tag(
            TriageFlagFilter::All,
            &rating_filter,
            false,
            false,
            Rating::TRASH_3,
            true,
        ));
        assert!(!filter_accepts_tag(
            TriageFlagFilter::All,
            &BTreeSet::from([3]),
            false,
            true,
            Rating::KEEP_3,
            true,
        ));
        assert!(filter_accepts_tag(
            TriageFlagFilter::All,
            &BTreeSet::from([3, 4]),
            false,
            true,
            Rating::KEEP_3,
            true,
        ));
    }

    #[test]
    fn folder_accepts_build_stops_when_generation_turns_stale() {
        let mut cache = SearchWorkerCache {
            entries: Some(
                (0..(super::super::SEARCH_CANCEL_CHECK_INTERVAL + 1))
                    .map(|index| CompactSearchEntry {
                        display_label: format!("item-{index}").into_boxed_str(),
                        relative_path: format!("group/item-{index}.wav").into_boxed_str(),
                        tag: Rating::NEUTRAL,
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

        let accepts = folder_accepts_for_job(
            &mut cache,
            &make_search_job("active"),
            "source-a",
            1,
            true,
            &queue,
            stale.generation,
        );

        assert!(accepts.is_none());
        assert!(cache.folder_accept_cache.is_empty());
    }

    fn make_search_job(query: &str) -> SearchJob {
        SearchJob {
            request_id: 1,
            source_id: SourceId::new(),
            source_root: PathBuf::from("root"),
            query: query.to_string(),
            filter: TriageFlagFilter::All,
            rating_filter: BTreeSet::new(),
            marked_only: false,
            marked_paths: BTreeSet::new(),
            sort: SampleBrowserSort::ListOrder,
            similar_query: None,
            duplicate_cleanup: None,
            folder_selection: Some(BTreeSet::from([PathBuf::from("group")])),
            folder_negated: None,
            file_scope_mode: crate::app::state::FolderFileScopeMode::AllDescendants,
        }
    }
}
