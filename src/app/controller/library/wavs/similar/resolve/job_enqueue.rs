//! Background analysis-job enqueue helpers for similarity refinement.

use crate::app::controller::AppController;
use rusqlite::{OptionalExtension, params};

/// Enqueue a full metadata analysis refresh when the current sample only has fast-prep data.
pub(super) fn maybe_enqueue_full_analysis(
    controller: &AppController,
    conn: &mut rusqlite::Connection,
    sample_id: &str,
) -> Result<(), String> {
    if !controller.similarity_prep_fast_mode_enabled() {
        return Ok(());
    }
    let row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT content_hash, analysis_version FROM samples WHERE sample_id = ?1",
            params![sample_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|err| format!("Load analysis version failed: {err}"))?;
    let Some((content_hash, analysis_version)) = row else {
        return Ok(());
    };
    if content_hash.trim().is_empty() {
        return Ok(());
    }
    let fast_version = crate::analysis::version::analysis_version_for_sample_rate(
        controller.similarity_prep_fast_sample_rate(),
    );
    if analysis_version.as_deref() != Some(fast_version.as_str()) {
        return Ok(());
    }
    let active: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM analysis_jobs
             WHERE sample_id = ?1 AND job_type = ?2 AND status IN ('pending','running')",
            params![sample_id, "wav_metadata_v1"],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if active > 0 {
        return Ok(());
    }
    let (source_id, relative_path) =
        crate::app::controller::library::analysis_jobs::parse_sample_id(sample_id)?;
    let relative_path = relative_path.to_string_lossy().replace('\\', "/");
    let created_at = now_epoch_seconds();
    conn.execute(
        "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 0, ?6)
         ON CONFLICT(sample_id, job_type) DO UPDATE SET
            source_id = excluded.source_id,
            relative_path = excluded.relative_path,
            content_hash = excluded.content_hash,
            status = 'pending',
            attempts = 0,
            created_at = excluded.created_at,
            last_error = NULL",
        params![
            sample_id,
            source_id,
            relative_path,
            "wav_metadata_v1",
            content_hash,
            created_at
        ],
    )
    .map_err(|err| format!("Enqueue analysis job failed: {err}"))?;
    Ok(())
}

fn now_epoch_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
