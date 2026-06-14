use super::super::telemetry;
use rusqlite::{Connection, params};

/// Typed inputs for inserting or replacing one embedding row.
pub(crate) struct EmbeddingUpsert<'a> {
    pub(crate) sample_id: &'a str,
    pub(crate) model_id: &'a str,
    pub(crate) dim: i64,
    pub(crate) dtype: &'a str,
    pub(crate) l2_normed: bool,
    pub(crate) vec_blob: &'a [u8],
    pub(crate) created_at: i64,
}

pub(crate) fn invalidate_analysis_artifacts(
    conn: &mut Connection,
    sample_ids: &[String],
) -> Result<(), String> {
    if sample_ids.is_empty() {
        return Ok(());
    }
    let tx = telemetry::begin_immediate_transaction(conn, "analysis_invalidation")
        .map_err(|err| format!("Failed to start analysis invalidation transaction: {err}"))?;
    invalidate_analysis_artifacts_in_tx(&tx, sample_ids)?;
    telemetry::commit_transaction(tx, "analysis_invalidation")
        .map_err(|err| format!("Failed to commit analysis invalidation transaction: {err}"))?;
    Ok(())
}

/// Remove persisted analysis artifacts inside an existing write transaction.
pub(crate) fn invalidate_analysis_artifacts_in_tx(
    tx: &rusqlite::Transaction<'_>,
    sample_ids: &[String],
) -> Result<(), String> {
    let mut stmt_features = tx
        .prepare("DELETE FROM features WHERE sample_id = ?1")
        .map_err(|err| format!("Failed to prepare analysis invalidation statement: {err}"))?;
    let mut stmt_embeddings = tx
        .prepare("DELETE FROM embeddings WHERE sample_id = ?1")
        .map_err(|err| format!("Failed to prepare analysis invalidation statement: {err}"))?;
    let mut stmt_legacy_features = tx
        .prepare("DELETE FROM analysis_features WHERE sample_id = ?1")
        .map_err(|err| format!("Failed to prepare analysis invalidation statement: {err}"))?;
    for sample_id in sample_ids {
        stmt_features
            .execute(params![sample_id])
            .map_err(|err| format!("Failed to invalidate analysis features: {err}"))?;
        stmt_embeddings
            .execute(params![sample_id])
            .map_err(|err| format!("Failed to invalidate embeddings: {err}"))?;
        stmt_legacy_features
            .execute(params![sample_id])
            .map_err(|err| format!("Failed to invalidate analysis features: {err}"))?;
    }
    drop(stmt_features);
    drop(stmt_embeddings);
    drop(stmt_legacy_features);
    Ok(())
}

pub(crate) fn upsert_analysis_features(
    conn: &Connection,
    sample_id: &str,
    vec_blob: &[u8],
    light_dsp_blob: Option<&[u8]>,
    rms: Option<f32>,
    feat_version: i64,
    computed_at: i64,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO features (sample_id, feat_version, vec_blob, light_dsp_blob, rms, computed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(sample_id) DO UPDATE SET
            feat_version = excluded.feat_version,
            vec_blob = excluded.vec_blob,
            light_dsp_blob = excluded.light_dsp_blob,
            rms = excluded.rms,
            computed_at = excluded.computed_at",
        params![
            sample_id,
            feat_version,
            vec_blob,
            light_dsp_blob,
            rms.map(f64::from),
            computed_at
        ],
    )
    .map_err(|err| format!("Failed to upsert analysis features: {err}"))?;
    Ok(())
}

pub(crate) fn upsert_embedding(
    conn: &Connection,
    embedding: EmbeddingUpsert<'_>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(sample_id) DO UPDATE SET
            model_id = excluded.model_id,
            dim = excluded.dim,
            dtype = excluded.dtype,
            l2_normed = excluded.l2_normed,
            vec = excluded.vec,
            created_at = excluded.created_at",
        params![
            embedding.sample_id,
            embedding.model_id,
            embedding.dim,
            embedding.dtype,
            embedding.l2_normed,
            embedding.vec_blob,
            embedding.created_at
        ],
    )
    .map_err(|err| format!("Failed to upsert embedding: {err}"))?;
    Ok(())
}
