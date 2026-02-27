//! Folder and tag filtering helpers for search worker pipeline execution.

use super::*;

/// Return whether a tag passes the active triage + rating filter settings.
pub(super) fn filter_accepts_tag(
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
