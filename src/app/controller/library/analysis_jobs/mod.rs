//! Background analysis job queue backed by the global library database.

pub(crate) mod db;
mod enqueue;
mod failures;
mod types;

#[cfg(test)]
pub(crate) use db::sample_bpm;
#[cfg(test)]
pub(crate) use db::update_sample_bpm;
pub(crate) use db::{
    AnalysisJobSession, AnalysisReadSession, open_source_db, open_source_db_background_read,
    open_source_db_maintenance, open_source_db_ui_read,
};
pub(crate) use db::{
    SampleMetadata, build_sample_id, parse_sample_id, update_sample_duration,
    update_sample_long_mark,
};
pub(crate) use enqueue::fast_content_hash;
pub(crate) use enqueue::update_missing_durations_for_source;
pub(crate) use failures::failed_samples_for_source;
pub(crate) use types::AnalysisJobMessage;

pub(crate) fn source_has_pending_or_running_jobs(
    source: &crate::sample_sources::SampleSource,
) -> Result<bool, String> {
    let current = crate::sample_sources::database_path_for(&source.root);
    let legacy = source
        .root
        .join(crate::sample_sources::db::LEGACY_DB_FILE_NAME);
    if !database_path_entry_present(&current)? && !database_path_entry_present(&legacy)? {
        return Ok(false);
    }
    let conn = db::open_source_db_maintenance(&source.root)?;
    db::has_pending_or_running_jobs(&conn)
}

fn database_path_entry_present(path: &std::path::Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "Failed to inspect source database path {}: {error}",
            path.display()
        )),
    }
}
