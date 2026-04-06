//! Search-worker source and compact-entry cache refresh helpers.

use super::super::*;
use std::hash::{Hash, Hasher};

/// Ensure the worker cache targets the job source, reopening DB/caches on source or stamp changes.
pub(in super::super) fn ensure_search_cache_ready_for_job(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    source_id: &str,
) -> bool {
    let db_path = crate::sample_sources::database_path_for(&job.source_root);
    let db_stamp = DbFileStamp::from_path(&db_path);
    let must_reopen = cache.db.is_none()
        || cache.source_id.as_deref() != Some(source_id)
        || cache.source_root.as_ref() != Some(&job.source_root)
        || cache.db_stamp.as_ref() != db_stamp.as_ref();
    if !must_reopen {
        return true;
    }
    let source_changed = cache.source_id.as_deref() != Some(source_id)
        || cache.source_root.as_ref() != Some(&job.source_root);

    match crate::sample_sources::SourceDatabase::open_read_only(&job.source_root) {
        Ok(db) => {
            cache.db = Some(db);
            cache.entries = None;
            cache.entry_lookup.clear();
            cache.revision = 0;
            cache.paths_revision = 0;
            if source_changed {
                cache.path_fingerprint = 0;
                cache.query_score_cache.clear();
            }
            cache.source_id = Some(source_id.to_string());
            cache.source_root = Some(job.source_root.clone());
            cache.db_stamp = db_stamp;
            cache.folder_accept_cache.clear();
            cache.filter_stage_cache.clear();
            cache.playback_age_token_caches.clear();
            cache.triage_cache = None;
            true
        }
        Err(_) => {
            cache.db = None;
            cache.entries = None;
            cache.entry_lookup.clear();
            cache.revision = 0;
            cache.paths_revision = 0;
            cache.path_fingerprint = 0;
            cache.source_id = Some(source_id.to_string());
            cache.source_root = Some(job.source_root.clone());
            cache.db_stamp = db_stamp;
            cache.query_score_cache.clear();
            cache.folder_accept_cache.clear();
            cache.filter_stage_cache.clear();
            cache.playback_age_token_caches.clear();
            cache.triage_cache = None;
            false
        }
    }
}

/// Load compact search entries when DB revision changes or cache is empty.
pub(in super::super) fn ensure_search_entries_loaded_for_job(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    queue: &SearchJobQueue,
    generation: u64,
) -> bool {
    let (revision, paths_revision) = {
        let Some(db) = cache.db.as_ref() else {
            return false;
        };
        (
            db.get_revision().unwrap_or(0),
            db.get_wav_paths_revision().unwrap_or(0),
        )
    };
    let must_reload = cache.entries.is_none() || cache.revision != revision;
    if !must_reload {
        return true;
    }

    if cache.entries.is_some() && cache.paths_revision == paths_revision {
        if !job.metadata_delta_paths.is_empty() {
            let delta_rows = {
                let Some(db) = cache.db.as_ref() else {
                    return false;
                };
                db.list_search_entry_rows_for_paths(&job.metadata_delta_paths)
                    .unwrap_or_default()
            };
            if refresh_cached_entry_metadata_delta(cache, &job.metadata_delta_paths, &delta_rows, revision)
            {
                return true;
            }
        }
        let metadata = {
            let Some(db) = cache.db.as_ref() else {
                return false;
            };
            match db.list_search_entry_metadata() {
                Ok(metadata) => metadata,
                Err(_) => Vec::new(),
            }
        };
        if refresh_cached_entry_metadata_only(cache, &metadata, revision) {
            return true;
        }
    }

    let rows = {
        let Some(db) = cache.db.as_ref() else {
            return false;
        };
        match db.list_search_entry_rows() {
            Ok(rows) => rows,
            Err(_) => {
                cache.entries = None;
                cache.revision = 0;
                cache.paths_revision = 0;
                cache.path_fingerprint = 0;
                cache.query_score_cache.clear();
                cache.folder_accept_cache.clear();
                cache.filter_stage_cache.clear();
                cache.playback_age_token_caches.clear();
                cache.triage_cache = None;
                return false;
            }
        }
    };

    reload_compact_entries(cache, &rows, queue, generation, revision, paths_revision)
}

fn reload_compact_entries(
    cache: &mut SearchWorkerCache,
    rows: &[crate::sample_sources::db::read::SearchEntryRow],
    queue: &SearchJobQueue,
    generation: u64,
    revision: u64,
    paths_revision: u64,
) -> bool {
    let Some(compact_entries) = build_compact_entries(rows, queue, generation) else {
        return false;
    };
    let Some(path_fingerprint) = hash_compact_entry_paths(&compact_entries, queue, generation)
    else {
        return false;
    };
    cache.entries = Some(compact_entries);
    cache.entry_lookup = build_entry_lookup(cache.entries.as_deref().unwrap_or(&[]));
    cache.revision = revision;
    cache.paths_revision = paths_revision;
    if cache.path_fingerprint != path_fingerprint {
        cache.path_fingerprint = path_fingerprint;
        cache.query_score_cache.clear();
    }
    cache.folder_accept_cache.clear();
    cache.filter_stage_cache.clear();
    cache.playback_age_token_caches.clear();
    cache.triage_cache = None;
    true
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
    cache.filter_stage_cache.clear();
    cache.playback_age_token_caches.clear();
    cache.triage_cache = None;
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
        let path = row.relative_path.to_string_lossy().to_string();
        let Some(index) = cache.entry_lookup.get(&path).copied() else {
            return false;
        };
        let Some(entry) = entries.get_mut(index) else {
            return false;
        };
        entry.tag = row.metadata.tag;
        entry.locked = row.metadata.locked;
        entry.last_played_at = row.metadata.last_played_at;
        updated = updated.saturating_add(1);
    }
    if updated == 0 {
        return false;
    }
    cache.revision = revision;
    cache.filter_stage_cache.clear();
    cache.playback_age_token_caches.clear();
    cache.triage_cache = None;
    true
}

fn hash_compact_entry_paths(
    entries: &[CompactSearchEntry],
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<u64> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (index, entry) in entries.iter().enumerate() {
        if search_job_canceled_for_index(queue, generation, index) {
            return None;
        }
        entry.relative_path.as_ref().hash(&mut hasher);
    }
    if search_job_canceled(queue, generation) {
        return None;
    }
    Some(hasher.finish())
}

fn refresh_cached_entry_metadata(
    cached_entries: &mut [CompactSearchEntry],
    metadata: &[crate::sample_sources::db::read::SearchEntryMetadata],
) {
    for (cached, metadata) in cached_entries.iter_mut().zip(metadata.iter()) {
        cached.tag = metadata.tag;
        cached.locked = metadata.locked;
        cached.last_played_at = metadata.last_played_at;
    }
}

fn build_entry_lookup(entries: &[CompactSearchEntry]) -> std::collections::HashMap<String, usize> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.relative_path.to_string(), index))
        .collect()
}

fn build_compact_entries(
    loaded_entries: &[crate::sample_sources::db::read::SearchEntryRow],
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<Vec<CompactSearchEntry>> {
    let mut compact_entries = Vec::with_capacity(loaded_entries.len());
    for (index, entry) in loaded_entries.iter().enumerate() {
        if search_job_canceled_for_index(queue, generation, index) {
            return None;
        }
        compact_entries.push(compact_search_entry_for(entry));
    }
    if search_job_canceled(queue, generation) {
        return None;
    }
    Some(compact_entries)
}

fn compact_search_entry_for(
    entry: &crate::sample_sources::db::read::SearchEntryRow,
) -> CompactSearchEntry {
    let relative_path = entry.relative_path.to_string_lossy().to_string();
    let display_label = crate::app::view_model::sample_display_label(&entry.relative_path);
    CompactSearchEntry {
        display_label: display_label.into_boxed_str(),
        relative_path: relative_path.into_boxed_str(),
        tag: entry.metadata.tag,
        locked: entry.metadata.locked,
        last_played_at: entry.metadata.last_played_at,
    }
}
