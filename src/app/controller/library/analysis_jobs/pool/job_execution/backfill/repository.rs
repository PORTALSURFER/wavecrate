//! Cache and repository helpers for embedding backfill planning.

use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::pool::job_execution::support::now_epoch_seconds;
use rusqlite::{OptionalExtension, params};

use super::model::EmbeddingData;

pub(super) fn cached_embedding_data(
    conn: &rusqlite::Connection,
    content_hash: &str,
    analysis_version: &str,
) -> Result<Option<EmbeddingData>, String> {
    let Some(cached) = db::cached_embedding_by_hash(
        conn,
        content_hash,
        analysis_version,
        crate::analysis::similarity::SIMILARITY_MODEL_ID,
    )?
    else {
        return Ok(None);
    };
    let Ok(vec) = crate::analysis::decode_f32_le_blob(&cached.vec_blob) else {
        return Ok(None);
    };
    if vec.len() != crate::analysis::similarity::SIMILARITY_DIM {
        return Ok(None);
    }
    Ok(Some(EmbeddingData {
        embedding: vec,
        created_at: cached.created_at,
    }))
}

pub(super) fn cached_feature_embedding_data(
    conn: &rusqlite::Connection,
    content_hash: &str,
    analysis_version: &str,
) -> Result<Option<EmbeddingData>, String> {
    let Some(cached) = db::cached_features_by_hash(
        conn,
        content_hash,
        analysis_version,
        crate::analysis::vector::FEATURE_VERSION_V1,
    )?
    else {
        return Ok(None);
    };
    let Ok(features) = crate::analysis::decode_f32_le_blob(&cached.vec_blob) else {
        return Ok(None);
    };
    let Ok(data) = embedding_data_from_features(&features) else {
        return Ok(None);
    };
    Ok(Some(data))
}

pub(super) fn embedding_data_from_features(features: &[f32]) -> Result<EmbeddingData, String> {
    let embedding = crate::analysis::similarity::embedding_from_features(features)?;
    Ok(EmbeddingData {
        embedding,
        created_at: now_epoch_seconds(),
    })
}

pub(super) fn load_features_vec_optional(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<Option<Vec<f32>>, String> {
    let blob: Option<Vec<u8>> = conn
        .query_row(
            "SELECT vec_blob FROM features WHERE sample_id = ?1",
            params![sample_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("Failed to load features for {sample_id}: {err}"))?;
    let Some(blob) = blob else {
        return Ok(None);
    };
    let vec = crate::analysis::decode_f32_le_blob(&blob)?;
    if vec.len() != crate::analysis::vector::FEATURE_VECTOR_LEN_V1 {
        return Ok(None);
    }
    Ok(Some(vec))
}
