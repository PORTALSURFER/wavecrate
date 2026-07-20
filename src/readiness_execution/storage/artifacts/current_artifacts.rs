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

/// Typed inputs for inserting or replacing one aspect descriptor row.
pub(crate) struct AspectDescriptorUpsert<'a> {
    pub(crate) sample_id: &'a str,
    pub(crate) model_id: &'a str,
    pub(crate) dim: i64,
    pub(crate) dtype: &'a str,
    pub(crate) l2_normed: bool,
    pub(crate) valid_mask: u32,
    pub(crate) vec_blob: &'a [u8],
    pub(crate) created_at: i64,
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

pub(crate) fn upsert_aspect_descriptors(
    conn: &Connection,
    descriptors: AspectDescriptorUpsert<'_>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO similarity_aspect_descriptors
            (sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(sample_id) DO UPDATE SET
            model_id = excluded.model_id,
            dim = excluded.dim,
            dtype = excluded.dtype,
            l2_normed = excluded.l2_normed,
            valid_mask = excluded.valid_mask,
            vec = excluded.vec,
            created_at = excluded.created_at",
        params![
            descriptors.sample_id,
            descriptors.model_id,
            descriptors.dim,
            descriptors.dtype,
            descriptors.l2_normed,
            i64::from(descriptors.valid_mask),
            descriptors.vec_blob,
            descriptors.created_at
        ],
    )
    .map_err(|err| format!("Failed to upsert aspect descriptors: {err}"))?;
    Ok(())
}
