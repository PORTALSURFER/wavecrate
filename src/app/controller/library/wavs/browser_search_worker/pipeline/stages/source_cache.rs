//! Search-worker source and compact-entry cache refresh helpers.

use super::super::*;
use crate::logging::{DbDebugEvent, emit_db_debug_event};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

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

    match crate::sample_sources::SourceDatabase::open_with_role(
        &job.source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::UiRead,
    ) {
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
        let revision = match db.get_revision() {
            Ok(value) => value,
            Err(err) => {
                record_search_cache_read_failure(job, "revision", &err.to_string());
                return false;
            }
        };
        let paths_revision = match db.get_wav_paths_revision() {
            Ok(value) => value,
            Err(err) => {
                record_search_cache_read_failure(job, "paths_revision", &err.to_string());
                return false;
            }
        };
        (revision, paths_revision)
    };
    let must_reload = cache.entries.is_none() || cache.revision != revision;
    if !must_reload {
        return true;
    }

    if cache.entries.is_some() && cache.paths_revision == paths_revision {
        let mut delta_read_failed = false;
        if !job.metadata_delta_paths.is_empty() {
            let delta_rows = match {
                let Some(db) = cache.db.as_ref() else {
                    return false;
                };
                db.list_search_entry_rows_for_paths(&job.metadata_delta_paths)
            } {
                Ok(rows) => rows,
                Err(err) => {
                    delta_read_failed = true;
                    record_search_cache_read_failure(job, "metadata_delta_rows", &err.to_string());
                    Vec::new()
                }
            };
            if !delta_read_failed
                && refresh_cached_entry_metadata_delta(
                    cache,
                    &job.metadata_delta_paths,
                    &delta_rows,
                    revision,
                )
            {
                return true;
            }
        }
        if !delta_read_failed {
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
            if refresh_cached_entry_metadata_only(cache, &metadata, revision) {
                return true;
            }
        }
    }

    let rows = {
        let Some(db) = cache.db.as_ref() else {
            return false;
        };
        match db.list_search_entry_rows() {
            Ok(rows) => rows,
            Err(err) => {
                record_search_cache_read_failure(job, "full_rows", &err.to_string());
                return false;
            }
        }
    };

    reload_compact_entries(cache, &rows, queue, generation, revision, paths_revision)
}

fn record_search_cache_read_failure(job: &SearchJob, read_type: &'static str, err: &str) {
    let source = job.source_root.display().to_string();
    tracing::warn!(
        target: "perf::source_db",
        action = "browser_search_cache_read_failed",
        read_type,
        source_id = job.source_id.as_str(),
        source_root = %job.source_root.display(),
        busy = is_busy_error(err),
        error = err,
        "Browser search cache read failed; preserving prior worker cache"
    );
    emit_db_debug_event(DbDebugEvent {
        operation: "browser_search_cache.read",
        source: Some(&source),
        outcome: "error",
        elapsed: std::time::Duration::ZERO,
        error: Some(err),
    });
}

fn is_busy_error(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("busy") || lowered.contains("locked")
}

fn reload_compact_entries(
    cache: &mut SearchWorkerCache,
    rows: &[crate::sample_sources::db::read::SearchEntryRow],
    queue: &SearchJobQueue,
    generation: u64,
    revision: u64,
    paths_revision: u64,
) -> bool {
    let Some((compact_entries, entry_lookup, path_fingerprint)) =
        build_compact_entries(rows, queue, generation)
    else {
        return false;
    };
    cache.entries = Some(compact_entries);
    cache.entry_lookup = entry_lookup;
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
            compact_search_display_label(&row.relative_path, &row.metadata).into_boxed_str();
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

fn build_compact_entries(
    loaded_entries: &[crate::sample_sources::db::read::SearchEntryRow],
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<(
    Vec<CompactSearchEntry>,
    std::collections::HashMap<Arc<str>, usize>,
    u64,
)> {
    let mut compact_entries = Vec::with_capacity(loaded_entries.len());
    let mut entry_lookup = std::collections::HashMap::with_capacity(loaded_entries.len());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (index, entry) in loaded_entries.iter().enumerate() {
        if search_job_canceled_for_index(queue, generation, index) {
            return None;
        }
        let compact_entry = compact_search_entry_for(entry);
        compact_entry.relative_path.as_ref().hash(&mut hasher);
        entry_lookup.insert(Arc::clone(&compact_entry.relative_path), index);
        compact_entries.push(compact_entry);
    }
    if search_job_canceled(queue, generation) {
        return None;
    }
    Some((compact_entries, entry_lookup, hasher.finish()))
}

fn compact_search_entry_for(
    entry: &crate::sample_sources::db::read::SearchEntryRow,
) -> CompactSearchEntry {
    let relative_path: Arc<str> = Arc::from(entry.relative_path.to_string_lossy().into_owned());
    let display_label = compact_search_display_label(&entry.relative_path, &entry.metadata);
    CompactSearchEntry {
        display_label: display_label.into_boxed_str(),
        relative_path,
        tag: entry.metadata.tag,
        locked: entry.metadata.locked,
        last_played_at: entry.metadata.last_played_at,
        tag_named: entry.metadata.tag_named,
    }
}

fn compact_search_display_label(
    relative_path: &std::path::Path,
    metadata: &crate::sample_sources::db::read::SearchEntryMetadata,
) -> String {
    let mut label = crate::app::view_model::sample_display_label(relative_path);
    for tag in &metadata.normal_tags {
        label.push(' ');
        label.push_str(tag);
    }
    label
}
