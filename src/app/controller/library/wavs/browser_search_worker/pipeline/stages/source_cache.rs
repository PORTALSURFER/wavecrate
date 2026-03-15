//! Search-worker source and compact-entry cache refresh helpers.

use super::super::*;

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

    match crate::sample_sources::SourceDatabase::open_read_only(&job.source_root) {
        Ok(db) => {
            cache.db = Some(db);
            cache.entries = None;
            cache.revision = 0;
            cache.source_id = Some(source_id.to_string());
            cache.source_root = Some(job.source_root.clone());
            cache.db_stamp = db_stamp;
            cache.query_score_cache.clear();
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            true
        }
        Err(_) => {
            cache.db = None;
            cache.entries = None;
            cache.revision = 0;
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
            let mut compact_entries = Vec::with_capacity(loaded_entries.len());
            for (index, entry) in loaded_entries.into_iter().enumerate() {
                if search_job_canceled_for_index(queue, generation, index) {
                    return false;
                }
                let relative_path = entry.relative_path.to_string_lossy().to_string();
                let display_label =
                    crate::app::view_model::sample_display_label(&entry.relative_path);
                compact_entries.push(CompactSearchEntry {
                    display_label: display_label.into_boxed_str(),
                    relative_path: relative_path.into_boxed_str(),
                    tag: entry.tag,
                    locked: entry.locked,
                    last_played_at: entry.last_played_at,
                });
            }
            if search_job_canceled(queue, generation) {
                return false;
            }
            cache.entries = Some(compact_entries);
            cache.revision = revision;
            cache.query_score_cache.clear();
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            true
        }
        Err(_) => {
            cache.entries = None;
            cache.query_score_cache.clear();
            cache.folder_accept_cache.clear();
            cache.triage_cache = None;
            false
        }
    }
}
