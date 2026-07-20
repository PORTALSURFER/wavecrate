//! Cache and repository helpers for embedding backfill planning.

use super::super::storage as db;
use rusqlite::{OptionalExtension, params};

use super::model::{AspectDescriptorData, EmbeddingData};

pub(super) fn cached_embedding_data(
    conn: &rusqlite::Connection,
    content_hash: &str,
    analysis_version: &str,
) -> Result<Option<EmbeddingData>, String> {
    let Some(cached) = db::cached_embedding_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
    )?
    else {
        return Ok(None);
    };
    let Ok(vec) = wavecrate_analysis::decode_f32_le_blob(&cached.vec_blob) else {
        return Ok(None);
    };
    if vec.len() != wavecrate_analysis::similarity::SIMILARITY_DIM {
        return Ok(None);
    }
    let Some(aspects) = cached_aspect_descriptor_data(conn, content_hash, analysis_version)? else {
        return Ok(None);
    };
    Ok(Some(EmbeddingData {
        embedding: vec,
        aspect_descriptors: aspects,
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
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
    )?
    else {
        return Ok(None);
    };
    let Ok(features) = wavecrate_analysis::decode_f32_le_blob(&cached.vec_blob) else {
        return Ok(None);
    };
    let Ok(data) = embedding_data_from_features(&features, cached.computed_at) else {
        return Ok(None);
    };
    Ok(Some(data))
}

pub(super) fn embedding_data_from_features(
    features: &[f32],
    created_at: i64,
) -> Result<EmbeddingData, String> {
    let embedding = wavecrate_analysis::similarity::embedding_from_features(features)?;
    let aspect_descriptors =
        wavecrate_analysis::aspects::aspect_descriptors_from_features_v1(features)?;
    Ok(EmbeddingData {
        embedding,
        aspect_descriptors: AspectDescriptorData {
            vec_blob: wavecrate_analysis::vector::encode_f32_le_blob(aspect_descriptors.packed()),
            valid_mask: aspect_descriptors.valid_mask(),
        },
        created_at,
    })
}

pub(super) fn cached_aspect_descriptor_data(
    conn: &rusqlite::Connection,
    content_hash: &str,
    analysis_version: &str,
) -> Result<Option<AspectDescriptorData>, String> {
    let Some(cached) = db::cached_aspect_descriptors_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
    )?
    else {
        return Ok(None);
    };
    if cached.dim != wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64
        || cached.dtype != wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32
        || !cached.l2_normed
        || cached.vec_blob.len() != wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM * 4
    {
        return Ok(None);
    }
    Ok(Some(AspectDescriptorData {
        vec_blob: cached.vec_blob,
        valid_mask: cached.valid_mask,
    }))
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
    let vec = wavecrate_analysis::decode_f32_le_blob(&blob)?;
    if vec.len() != wavecrate_analysis::vector::FEATURE_VECTOR_LEN_V1 {
        return Ok(None);
    }
    Ok(Some(vec))
}

pub(super) fn sample_has_current_aspect_descriptors(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM similarity_aspect_descriptors
             WHERE sample_id = ?1
               AND model_id = ?2
               AND dim = ?3
               AND dtype = ?4
               AND l2_normed = 1",
            params![
                sample_id,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            ],
            |row| row.get(0),
        )
        .map_err(|err| format!("Failed to load aspect coverage for {sample_id}: {err}"))?;
    Ok(count > 0)
}
