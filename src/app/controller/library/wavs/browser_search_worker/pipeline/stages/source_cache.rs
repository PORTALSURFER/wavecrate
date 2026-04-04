//! Search-worker source and compact-entry cache refresh helpers.

use super::super::*;
use std::hash::{Hash, Hasher};
use std::path::Path;

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
            cache.revision = 0;
            if source_changed {
                cache.path_fingerprint = 0;
                cache.query_score_cache.clear();
            }
            cache.source_id = Some(source_id.to_string());
            cache.source_root = Some(job.source_root.clone());
            cache.db_stamp = db_stamp;
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            true
        }
        Err(_) => {
            cache.db = None;
            cache.entries = None;
            cache.revision = 0;
            cache.path_fingerprint = 0;
            cache.source_id = Some(source_id.to_string());
            cache.source_root = Some(job.source_root.clone());
            cache.db_stamp = db_stamp;
            cache.query_score_cache.clear();
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            false
        }
    }
}

/// Load compact search entries when DB revision changes or cache is empty.
pub(in super::super) fn ensure_search_entries_loaded_for_job(
    cache: &mut SearchWorkerCache,
    _job: &SearchJob,
    queue: &SearchJobQueue,
    generation: u64,
) -> bool {
    let Some(db) = cache.db.as_ref() else {
        return false;
    };
    let revision = db.get_revision().unwrap_or(0);
    let must_reload = cache.entries.is_none() || cache.revision != revision;
    if !must_reload {
        return true;
    }

    match db.list_files() {
        Ok(loaded_entries) => {
            let Some(path_fingerprint) =
                hash_loaded_entry_paths(&loaded_entries, queue, generation)
            else {
                return false;
            };
            if loaded_entries_match_cached_paths(cache.entries.as_deref(), &loaded_entries) {
                if let Some(entries) = cache.entries.as_mut() {
                    refresh_cached_entry_metadata(entries, &loaded_entries);
                }
            } else {
                let Some(compact_entries) =
                    build_compact_entries(&loaded_entries, queue, generation)
                else {
                    return false;
                };
                cache.entries = Some(compact_entries);
            }
            cache.revision = revision;
            if cache.path_fingerprint != path_fingerprint {
                cache.path_fingerprint = path_fingerprint;
                cache.query_score_cache.clear();
            }
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            true
        }
        Err(_) => {
            cache.entries = None;
            cache.path_fingerprint = 0;
            cache.query_score_cache.clear();
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            false
        }
    }
}

fn hash_loaded_entry_paths(
    loaded_entries: &[crate::sample_sources::WavEntry],
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<u64> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (index, entry) in loaded_entries.iter().enumerate() {
        if search_job_canceled_for_index(queue, generation, index) {
            return None;
        }
        entry.relative_path.hash(&mut hasher);
    }
    if search_job_canceled(queue, generation) {
        return None;
    }
    Some(hasher.finish())
}

fn loaded_entries_match_cached_paths(
    cached_entries: Option<&[CompactSearchEntry]>,
    loaded_entries: &[crate::sample_sources::WavEntry],
) -> bool {
    let Some(cached_entries) = cached_entries else {
        return false;
    };
    cached_entries.len() == loaded_entries.len()
        && cached_entries
            .iter()
            .zip(loaded_entries.iter())
            .all(|(cached, loaded)| {
                Path::new(cached.relative_path.as_ref()) == loaded.relative_path.as_path()
            })
}

fn refresh_cached_entry_metadata(
    cached_entries: &mut [CompactSearchEntry],
    loaded_entries: &[crate::sample_sources::WavEntry],
) {
    for (cached, loaded) in cached_entries.iter_mut().zip(loaded_entries.iter()) {
        cached.tag = loaded.tag;
        cached.locked = loaded.locked;
        cached.last_played_at = loaded.last_played_at;
    }
}

fn build_compact_entries(
    loaded_entries: &[crate::sample_sources::WavEntry],
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

fn compact_search_entry_for(entry: &crate::sample_sources::WavEntry) -> CompactSearchEntry {
    let relative_path = entry.relative_path.to_string_lossy().to_string();
    let display_label = crate::app::view_model::sample_display_label(&entry.relative_path);
    CompactSearchEntry {
        display_label: display_label.into_boxed_str(),
        relative_path: relative_path.into_boxed_str(),
        tag: entry.tag,
        locked: entry.locked,
        last_played_at: entry.last_played_at,
    }
}
