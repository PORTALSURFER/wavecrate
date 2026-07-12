use super::super::super::*;
use super::cache_invalidation::{
    clear_derived_search_caches, clear_source_identity_caches, reset_entries_for_db_reopen,
};

/// Ensure the worker cache targets the job source, reopening DB/caches on source or stamp changes.
pub(in super::super::super) fn ensure_search_cache_ready_for_job(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    source_id: &str,
) -> bool {
    let must_reopen = cache.db.is_none()
        || cache.source_id.as_deref() != Some(source_id)
        || cache.source_root.as_ref() != Some(&job.source_root);
    if !must_reopen {
        return true;
    }

    let db_path = crate::sample_sources::database_path_for(&job.source_root);
    let db_stamp = DbFileStamp::from_path(&db_path);
    let source_changed = cache.source_id.as_deref() != Some(source_id)
        || cache.source_root.as_ref() != Some(&job.source_root);
    match crate::sample_sources::SourceDatabase::open_for_ui_read(&job.source_root) {
        Ok(db) => {
            cache.db = Some(db);
            apply_reopened_source_db(cache, job, source_id, db_stamp, source_changed);
            true
        }
        Err(_) => {
            cache.db = None;
            apply_failed_source_db_open(cache, job, source_id, db_stamp);
            false
        }
    }
}

fn apply_reopened_source_db(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    source_id: &str,
    db_stamp: Option<DbFileStamp>,
    source_changed: bool,
) {
    reset_entries_for_db_reopen(cache);
    if source_changed {
        clear_source_identity_caches(cache);
    }
    cache.source_id = Some(source_id.to_string());
    cache.source_root = Some(job.source_root.clone());
    cache.db_stamp = db_stamp;
    clear_derived_search_caches(cache);
}

fn apply_failed_source_db_open(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    source_id: &str,
    db_stamp: Option<DbFileStamp>,
) {
    reset_entries_for_db_reopen(cache);
    clear_source_identity_caches(cache);
    cache.source_id = Some(source_id.to_string());
    cache.source_root = Some(job.source_root.clone());
    cache.db_stamp = db_stamp;
    clear_derived_search_caches(cache);
}
