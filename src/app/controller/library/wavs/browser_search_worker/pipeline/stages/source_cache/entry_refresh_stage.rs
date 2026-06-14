use super::super::super::*;
use super::cache_invalidation::clear_metadata_dependent_caches;
use super::compact_entries::reload_compact_entries;
use super::read_failures::record_search_cache_read_failure;
use std::path::PathBuf;

/// Load compact search entries when DB revision changes or cache is empty.
pub(in super::super::super) fn ensure_search_entries_loaded_for_job(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    queue: &SearchJobQueue,
    generation: u64,
) -> bool {
    let Some((revision, paths_revision)) = read_source_revisions(cache, job) else {
        return false;
    };
    if cache.entries.is_some() && cache.revision == revision {
        return true;
    }

    if try_refresh_entries_without_path_changes(cache, job, revision, paths_revision) {
        return true;
    }

    let Some(rows) = read_full_search_rows(cache, job) else {
        return false;
    };
    reload_compact_entries(cache, &rows, queue, generation, revision, paths_revision)
}

fn read_source_revisions(cache: &SearchWorkerCache, job: &SearchJob) -> Option<(u64, u64)> {
    let db = cache.db.as_ref()?;
    let revision = match db.get_revision() {
        Ok(value) => value,
        Err(err) => {
            record_search_cache_read_failure(job, "revision", &err.to_string());
            return None;
        }
    };
    let paths_revision = match db.get_wav_paths_revision() {
        Ok(value) => value,
        Err(err) => {
            record_search_cache_read_failure(job, "paths_revision", &err.to_string());
            return None;
        }
    };
    Some((revision, paths_revision))
}

fn try_refresh_entries_without_path_changes(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    revision: u64,
    paths_revision: u64,
) -> bool {
    if cache.entries.is_none() || cache.paths_revision != paths_revision {
        return false;
    }

    if !job.metadata_delta_paths.is_empty() {
        match try_refresh_targeted_metadata_delta(cache, job, revision) {
            DeltaRefreshOutcome::Refreshed => return true,
            DeltaRefreshOutcome::NeedsMetadataOnly => {}
            DeltaRefreshOutcome::ReadFailed => return false,
        }
    }
    try_refresh_metadata_only(cache, job, revision)
}

enum DeltaRefreshOutcome {
    Refreshed,
    NeedsMetadataOnly,
    ReadFailed,
}

fn try_refresh_targeted_metadata_delta(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    revision: u64,
) -> DeltaRefreshOutcome {
    let Some(db) = cache.db.as_ref() else {
        return DeltaRefreshOutcome::ReadFailed;
    };
    let delta_rows = match db.list_search_entry_rows_for_paths(&job.metadata_delta_paths) {
        Ok(rows) => rows,
        Err(err) => {
            record_search_cache_read_failure(job, "metadata_delta_rows", &err.to_string());
            return DeltaRefreshOutcome::ReadFailed;
        }
    };
    if refresh_cached_entry_metadata_delta(cache, &job.metadata_delta_paths, &delta_rows, revision)
    {
        DeltaRefreshOutcome::Refreshed
    } else {
        DeltaRefreshOutcome::NeedsMetadataOnly
    }
}

fn try_refresh_metadata_only(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    revision: u64,
) -> bool {
    let Some(db) = cache.db.as_ref() else {
        return false;
    };
    let metadata = match db.list_search_entry_metadata() {
        Ok(metadata) => metadata,
        Err(err) => {
            record_search_cache_read_failure(job, "metadata_rows", &err.to_string());
            return false;
        }
    };
    refresh_cached_entry_metadata_only(cache, &metadata, revision)
}

fn read_full_search_rows(
    cache: &SearchWorkerCache,
    job: &SearchJob,
) -> Option<Vec<crate::sample_sources::db::read::SearchEntryRow>> {
    let db = cache.db.as_ref()?;
    match db.list_search_entry_rows() {
        Ok(rows) => Some(rows),
        Err(err) => {
            record_search_cache_read_failure(job, "full_rows", &err.to_string());
            None
        }
    }
}

fn refresh_cached_entry_metadata_only(
    cache: &mut SearchWorkerCache,
    metadata: &[crate::sample_sources::db::read::SearchEntryMetadata],
    revision: u64,
) -> bool {
    let Some(entries) = cache.entries.as_mut() else {
        return false;
    };
    if entries.len() != metadata.len() {
        return false;
    }
    refresh_cached_entry_metadata(entries, metadata);
    cache.revision = revision;
    clear_metadata_dependent_caches(cache);
    true
}

fn refresh_cached_entry_metadata_delta(
    cache: &mut SearchWorkerCache,
    delta_paths: &[PathBuf],
    rows: &[crate::sample_sources::db::read::SearchEntryRow],
    revision: u64,
) -> bool {
    let Some(entries) = cache.entries.as_mut() else {
        return false;
    };
    if rows.is_empty() || rows.len() > delta_paths.len() {
        return false;
    }
    let mut updated = 0usize;
    for row in rows {
        let path = row.relative_path.to_string_lossy();
        let Some(index) = cache.entry_lookup.get(path.as_ref()).copied() else {
            return false;
        };
        let Some(entry) = entries.get_mut(index) else {
            return false;
        };
        entry.tag = row.metadata.tag;
        entry.locked = row.metadata.locked;
        entry.last_played_at = row.metadata.last_played_at;
        entry.tag_named = row.metadata.tag_named;
        entry.display_label =
            super::compact_entries::compact_search_display_label(&row.relative_path, &row.metadata)
                .into_boxed_str();
        updated = updated.saturating_add(1);
    }
    if updated == 0 {
        return false;
    }
    cache.revision = revision;
    clear_metadata_dependent_caches(cache);
    true
}

fn refresh_cached_entry_metadata(
    cached_entries: &mut [CompactSearchEntry],
    metadata: &[crate::sample_sources::db::read::SearchEntryMetadata],
) {
    for (cached, metadata) in cached_entries.iter_mut().zip(metadata.iter()) {
        cached.tag = metadata.tag;
        cached.locked = metadata.locked;
        cached.last_played_at = metadata.last_played_at;
        cached.tag_named = metadata.tag_named;
    }
}
