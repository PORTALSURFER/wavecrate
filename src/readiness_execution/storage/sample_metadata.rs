#[cfg(test)]
use super::telemetry;
#[cfg(any(test, feature = "legacy-controller"))]
use rusqlite::params_from_iter;
use rusqlite::{Connection, OptionalExtension, params};
#[cfg(any(test, feature = "legacy-controller"))]
use std::collections::HashSet;

pub(crate) fn sample_content_hash(
    conn: &Connection,
    sample_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT content_hash FROM samples WHERE sample_id = ?1",
        params![sample_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| format!("Failed to lookup sample content hash: {err}"))
}

/// Load the stored BPM for a sample, if present.
#[cfg(test)]
pub(crate) fn sample_bpm(conn: &Connection, sample_id: &str) -> Result<Option<f32>, String> {
    let bpm: Option<f64> = conn
        .query_row(
            "SELECT bpm FROM samples WHERE sample_id = ?1",
            params![sample_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("Failed to lookup sample bpm: {err}"))?
        .flatten();
    Ok(bpm
        .map(|value| value as f32)
        .filter(|value| value.is_finite() && *value > 0.0))
}

/// Update the stored BPM for a sample row, clearing it if the value is invalid.
#[cfg(test)]
pub(crate) fn update_sample_bpm(
    conn: &Connection,
    sample_id: &str,
    bpm: Option<f32>,
) -> Result<(), String> {
    let bpm = normalized_bpm(bpm);
    let updated = conn
        .execute(
            "UPDATE samples SET bpm = ?2 WHERE sample_id = ?1",
            params![sample_id, bpm],
        )
        .map_err(|err| format!("Failed to update sample bpm: {err}"))?;
    if updated == 0 {
        return Err(format!("No sample row updated for sample_id={sample_id}"));
    }
    Ok(())
}

/// Update the stored BPM for multiple sample rows, clearing it if the value is invalid.
#[cfg(test)]
pub(crate) fn update_sample_bpms(
    conn: &mut Connection,
    sample_ids: &[String],
    bpm: Option<f32>,
) -> Result<usize, String> {
    if sample_ids.is_empty() {
        return Ok(0);
    }
    let tx = telemetry::begin_immediate_transaction(conn, "analysis_update_bpms")
        .map_err(|err| format!("Failed to start BPM update transaction: {err}"))?;
    let updated = update_sample_bpms_in_tx(&tx, sample_ids, normalized_bpm(bpm))?;
    telemetry::commit_transaction(tx, "analysis_update_bpms")
        .map_err(|err| format!("Failed to commit BPM updates: {err}"))?;
    Ok(updated)
}

/// Update stored BPM values for multiple samples inside an existing write transaction.
#[cfg(any(test, feature = "legacy-controller"))]
pub(crate) fn update_sample_bpms_in_tx(
    conn: &rusqlite::Transaction<'_>,
    sample_ids: &[String],
    bpm: Option<f64>,
) -> Result<usize, String> {
    let mut updated = 0usize;
    for sample_id in sample_ids {
        let count = conn
            .execute(
                "UPDATE samples SET bpm = ?2 WHERE sample_id = ?1",
                params![sample_id, bpm],
            )
            .map_err(|err| format!("Failed to update sample bpm: {err}"))?;
        if count == 0 {
            return Err(format!("No sample row updated for sample_id={sample_id}"));
        }
        updated = updated.saturating_add(count);
    }
    Ok(updated)
}

/// Return the subset of sample ids that lack a stored duration.
#[cfg(any(test, feature = "legacy-controller"))]
pub(crate) fn sample_ids_missing_duration(
    conn: &Connection,
    sample_ids: &[String],
) -> Result<HashSet<String>, String> {
    let mut missing = HashSet::new();
    if sample_ids.is_empty() {
        return Ok(missing);
    }
    let placeholders = placeholders(sample_ids.len());
    let sql = format!(
        "SELECT sample_id
         FROM samples
         WHERE sample_id IN ({placeholders})
           AND (duration_seconds IS NULL OR duration_seconds <= 0)"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("Failed to prepare duration lookup: {err}"))?;
    let mut rows = stmt
        .query(params_from_iter(sample_ids.iter()))
        .map_err(|err| format!("Failed to query duration metadata: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Failed to query duration metadata: {err}"))?
    {
        let sample_id: String = row.get(0).map_err(|err| err.to_string())?;
        missing.insert(sample_id);
    }
    Ok(missing)
}

#[cfg(test)]
fn normalized_bpm(bpm: Option<f32>) -> Option<f64> {
    bpm.filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value as f64)
}

#[cfg(any(test, feature = "legacy-controller"))]
fn placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(", ")
}
