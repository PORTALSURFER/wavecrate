use crate::app::controller::library::analysis_jobs::db;

use super::super::support::now_epoch_seconds;

pub(super) fn upsert_ann_with_recovery(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    embedding: &[f32],
) -> Result<(), String> {
    if let Err(err) = crate::analysis::ann_index::upsert_embedding(conn, &job.sample_id, embedding)
    {
        let rebuild_result = handle_ann_update_failure(conn, job, &err);
        return Err(format_ann_update_error(err, rebuild_result));
    }
    Ok(())
}

fn handle_ann_update_failure(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    err: &str,
) -> Result<(), String> {
    let (source_id, _relative) = db::parse_sample_id(&job.sample_id)?;
    db::mark_ann_index_dirty(conn, err)?;
    db::enqueue_rebuild_ann_index_job(conn, &source_id, now_epoch_seconds())?;
    Ok(())
}

fn format_ann_update_error(err: String, rebuild_result: Result<(), String>) -> String {
    match rebuild_result {
        Ok(()) => format!("ANN index update failed; rebuild scheduled: {err}"),
        Err(rebuild_err) => format!(
            "ANN index update failed; rebuild scheduling failed: {rebuild_err}; original error: {err}"
        ),
    }
}
