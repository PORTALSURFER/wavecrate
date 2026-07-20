use rusqlite::{Connection, params};

/// Typed inputs for updating duration/sample-rate metadata on an existing sample row.
pub(crate) struct AnalysisMetadataUpdate<'a> {
    pub(crate) sample_id: &'a str,
    pub(crate) content_hash: Option<&'a str>,
    pub(crate) duration_seconds: f32,
    pub(crate) sr_used: u32,
    pub(crate) analysis_version: &'a str,
}

pub(crate) fn update_analysis_metadata(
    conn: &Connection,
    update: AnalysisMetadataUpdate<'_>,
) -> Result<(), String> {
    let updated = conn
        .execute(
            "UPDATE samples
             SET duration_seconds = ?3, sr_used = ?4, analysis_version = ?5
             WHERE sample_id = ?1 AND content_hash = COALESCE(?2, content_hash)",
            params![
                update.sample_id,
                update.content_hash,
                update.duration_seconds as f64,
                update.sr_used as i64,
                update.analysis_version
            ],
        )
        .map_err(|err| format!("Failed to update analysis metadata: {err}"))?;
    if updated == 0 {
        return Err(format!(
            "No sample row updated for sample_id={}",
            update.sample_id
        ));
    }
    Ok(())
}

/// Update duration/sample rate metadata without changing analysis version.
/// Returns true when the duration was updated.
#[cfg(any(test, feature = "legacy-controller"))]
pub(crate) fn update_sample_duration(
    conn: &Connection,
    sample_id: &str,
    duration_seconds: f32,
    sr_used: u32,
) -> Result<bool, String> {
    let updated = conn
        .execute(
            "UPDATE samples
             SET duration_seconds = ?2, sr_used = ?3
             WHERE sample_id = ?1
               AND (duration_seconds IS NULL OR duration_seconds <= 0)",
            params![sample_id, duration_seconds as f64, sr_used as i64],
        )
        .map_err(|err| format!("Failed to update sample duration: {err}"))?;
    Ok(updated > 0)
}

/// Persist the long-sample marker for a sample row.
#[cfg(any(test, feature = "legacy-controller"))]
pub(crate) fn update_sample_long_mark(
    conn: &Connection,
    sample_id: &str,
    long_sample_mark: bool,
) -> Result<(), String> {
    let mark = if long_sample_mark { 1i64 } else { 0i64 };
    conn.execute(
        "UPDATE samples SET long_sample_mark = ?2 WHERE sample_id = ?1",
        params![sample_id, mark],
    )
    .map_err(|err| format!("Failed to update long sample mark: {err}"))?;
    Ok(())
}
