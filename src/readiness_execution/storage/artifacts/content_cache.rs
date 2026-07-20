use rusqlite::{Connection, OptionalExtension, params};

pub(crate) struct CachedFeatures {
    pub(crate) feat_version: i64,
    pub(crate) vec_blob: Vec<u8>,
    pub(crate) light_dsp_blob: Option<Vec<u8>>,
    pub(crate) rms: Option<f32>,
    pub(crate) computed_at: i64,
    pub(crate) duration_seconds: f32,
    pub(crate) sr_used: u32,
}

pub(crate) struct CachedEmbedding {
    pub(crate) vec_blob: Vec<u8>,
    pub(crate) created_at: i64,
}

pub(crate) struct CachedAspectDescriptors {
    pub(crate) dim: i64,
    pub(crate) dtype: String,
    pub(crate) l2_normed: bool,
    pub(crate) valid_mask: u32,
    pub(crate) vec_blob: Vec<u8>,
}

/// Typed inputs for caching reusable feature vectors by content hash.
pub(crate) struct CachedFeaturesUpsert<'a> {
    pub(crate) content_hash: &'a str,
    pub(crate) analysis_version: &'a str,
    pub(crate) feat_version: i64,
    pub(crate) vec_blob: &'a [u8],
    pub(crate) light_dsp_blob: Option<&'a [u8]>,
    pub(crate) rms: Option<f32>,
    pub(crate) computed_at: i64,
    pub(crate) duration_seconds: f32,
    pub(crate) sr_used: u32,
}

/// Typed inputs for caching reusable embeddings by content hash and model.
pub(crate) struct CachedEmbeddingUpsert<'a> {
    pub(crate) content_hash: &'a str,
    pub(crate) analysis_version: &'a str,
    pub(crate) model_id: &'a str,
    pub(crate) dim: i64,
    pub(crate) dtype: &'a str,
    pub(crate) l2_normed: bool,
    pub(crate) vec_blob: &'a [u8],
    pub(crate) created_at: i64,
}

/// Typed inputs for caching reusable aspect descriptors by content hash and model.
pub(crate) struct CachedAspectDescriptorsUpsert<'a> {
    pub(crate) content_hash: &'a str,
    pub(crate) analysis_version: &'a str,
    pub(crate) model_id: &'a str,
    pub(crate) dim: i64,
    pub(crate) dtype: &'a str,
    pub(crate) l2_normed: bool,
    pub(crate) valid_mask: u32,
    pub(crate) vec_blob: &'a [u8],
    pub(crate) created_at: i64,
}

pub(crate) fn cached_features_by_hash(
    conn: &Connection,
    content_hash: &str,
    analysis_version: &str,
    feat_version: i64,
) -> Result<Option<CachedFeatures>, String> {
    conn.query_row(
        "SELECT feat_version, vec_blob, light_dsp_blob, rms, computed_at, duration_seconds, sr_used
         FROM analysis_cache_features
         WHERE content_hash = ?1 AND analysis_version = ?2 AND feat_version = ?3",
        params![content_hash, analysis_version, feat_version],
        |row| {
            Ok(CachedFeatures {
                feat_version: row.get(0)?,
                vec_blob: row.get(1)?,
                light_dsp_blob: row.get(2)?,
                rms: row.get::<_, Option<f64>>(3)?.map(|value| value as f32),
                computed_at: row.get(4)?,
                duration_seconds: row.get::<_, f64>(5)? as f32,
                sr_used: row.get::<_, i64>(6)? as u32,
            })
        },
    )
    .optional()
    .map_err(|err| format!("Failed to load cached features for {content_hash}: {err}"))
}

pub(crate) fn cached_embedding_by_hash(
    conn: &Connection,
    content_hash: &str,
    analysis_version: &str,
    model_id: &str,
) -> Result<Option<CachedEmbedding>, String> {
    conn.query_row(
        "SELECT vec, created_at
         FROM analysis_cache_embeddings
         WHERE content_hash = ?1 AND analysis_version = ?2 AND model_id = ?3",
        params![content_hash, analysis_version, model_id],
        |row| {
            Ok(CachedEmbedding {
                vec_blob: row.get(0)?,
                created_at: row.get(1)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("Failed to load cached embedding for {content_hash}: {err}"))
}

pub(crate) fn cached_aspect_descriptors_by_hash(
    conn: &Connection,
    content_hash: &str,
    analysis_version: &str,
    model_id: &str,
) -> Result<Option<CachedAspectDescriptors>, String> {
    conn.query_row(
        "SELECT dim, dtype, l2_normed, valid_mask, vec
         FROM analysis_cache_aspect_descriptors
         WHERE content_hash = ?1 AND analysis_version = ?2 AND model_id = ?3",
        params![content_hash, analysis_version, model_id],
        |row| {
            Ok(CachedAspectDescriptors {
                dim: row.get(0)?,
                dtype: row.get(1)?,
                l2_normed: row.get::<_, i64>(2)? != 0,
                valid_mask: row.get::<_, i64>(3)? as u32,
                vec_blob: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("Failed to load cached aspect descriptors for {content_hash}: {err}"))
}

pub(crate) fn upsert_cached_features(
    conn: &Connection,
    features: CachedFeaturesUpsert<'_>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO analysis_cache_features
            (content_hash, analysis_version, feat_version, vec_blob, light_dsp_blob, rms, computed_at, duration_seconds, sr_used)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(content_hash) DO UPDATE SET
            analysis_version = excluded.analysis_version,
            feat_version = excluded.feat_version,
            vec_blob = excluded.vec_blob,
            light_dsp_blob = excluded.light_dsp_blob,
            rms = excluded.rms,
            computed_at = excluded.computed_at,
            duration_seconds = excluded.duration_seconds,
            sr_used = excluded.sr_used",
        params![
            features.content_hash,
            features.analysis_version,
            features.feat_version,
            features.vec_blob,
            features.light_dsp_blob,
            features.rms.map(f64::from),
            features.computed_at,
            features.duration_seconds as f64,
            features.sr_used as i64
        ],
    )
    .map_err(|err| format!("Failed to upsert cached features: {err}"))?;
    Ok(())
}

pub(crate) fn upsert_cached_embedding(
    conn: &Connection,
    embedding: CachedEmbeddingUpsert<'_>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO analysis_cache_embeddings
            (content_hash, analysis_version, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(content_hash, model_id) DO UPDATE SET
            analysis_version = excluded.analysis_version,
            dim = excluded.dim,
            dtype = excluded.dtype,
            l2_normed = excluded.l2_normed,
            vec = excluded.vec,
            created_at = excluded.created_at",
        params![
            embedding.content_hash,
            embedding.analysis_version,
            embedding.model_id,
            embedding.dim,
            embedding.dtype,
            embedding.l2_normed,
            embedding.vec_blob,
            embedding.created_at
        ],
    )
    .map_err(|err| format!("Failed to upsert cached embedding: {err}"))?;
    Ok(())
}

pub(crate) fn upsert_cached_aspect_descriptors(
    conn: &Connection,
    descriptors: CachedAspectDescriptorsUpsert<'_>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO analysis_cache_aspect_descriptors
            (content_hash, analysis_version, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(content_hash, model_id) DO UPDATE SET
            analysis_version = excluded.analysis_version,
            dim = excluded.dim,
            dtype = excluded.dtype,
            l2_normed = excluded.l2_normed,
            valid_mask = excluded.valid_mask,
            vec = excluded.vec,
            created_at = excluded.created_at",
        params![
            descriptors.content_hash,
            descriptors.analysis_version,
            descriptors.model_id,
            descriptors.dim,
            descriptors.dtype,
            descriptors.l2_normed,
            i64::from(descriptors.valid_mask),
            descriptors.vec_blob,
            descriptors.created_at
        ],
    )
    .map_err(|err| format!("Failed to upsert cached aspect descriptors: {err}"))?;
    Ok(())
}
