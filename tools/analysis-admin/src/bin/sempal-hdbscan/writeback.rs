//! Database writeback helpers for HDBSCAN cluster assignments.

use rusqlite::{Connection, params};
use std::time::{SystemTime, UNIX_EPOCH};

/// Persist cluster labels back into the analysis database.
pub(super) fn write_clusters(
    conn: &mut Connection,
    sample_ids: &[String],
    labels: &[i32],
    model_id: &str,
    method: &str,
    umap_version: &str,
) -> Result<usize, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "Invalid system time".to_string())?
        .as_secs() as i64;
    let tx = conn
        .transaction()
        .map_err(|err| format!("Start transaction failed: {err}"))?;
    let mut stmt = tx
        .prepare(
            "INSERT INTO hdbscan_clusters (
                sample_id,
                model_id,
                method,
                umap_version,
                cluster_id,
                created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(sample_id, model_id, method, umap_version) DO UPDATE SET
                cluster_id = excluded.cluster_id,
                created_at = excluded.created_at",
        )
        .map_err(|err| format!("Prepare cluster insert failed: {err}"))?;
    for (idx, sample_id) in sample_ids.iter().enumerate() {
        let label = labels
            .get(idx)
            .ok_or_else(|| "Cluster label length mismatch".to_string())?;
        stmt.execute(params![
            sample_id,
            model_id,
            method,
            umap_version,
            label,
            now
        ])
        .map_err(|err| format!("Insert cluster failed: {err}"))?;
    }
    drop(stmt);
    tx.commit()
        .map_err(|err| format!("Commit clusters failed: {err}"))?;
    Ok(sample_ids.len())
}
